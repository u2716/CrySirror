// SPDX-License-Identifier: MPL-2.0
//! Native newest-first dialogue history.

use super::{
    abi::StringRef,
    evaluate::{error, Journal},
    machine::Machine,
    DecodeError,
};
use std::collections::VecDeque;

const CAPACITY: usize = 128;

#[derive(Clone)]
struct Entry {
    strings: [Vec<u8>; 4],
    words: [u32; 6],
}

#[derive(Clone)]
pub(super) struct History {
    entries: VecDeque<Entry>,
    save_limit: u32,
    /// First word of the history object (+4328 → +0), written by `0x2137`, read by
    /// `0x212e` and stored with slot saves (0x1400730b0).
    word: u32,
}

impl Default for History {
    fn default() -> Self {
        Self {
            entries: VecDeque::new(),
            save_limit: 32,
            word: 0,
        }
    }
}

#[cfg(feature = "runtime")]
impl History {
    pub(super) fn snapshot(&self) -> serde_json::Value {
        use super::snapshot::hex;
        let entries: Vec<_> = self
            .entries
            .iter()
            .map(|entry| {
                let strings: Vec<_> = entry.strings.iter().map(|text| hex(text)).collect();
                serde_json::json!({"strings": strings, "words": entry.words})
            })
            .collect();
        serde_json::json!({"entries": entries, "save_limit": self.save_limit, "word": self.word})
    }

    pub(super) fn from_snapshot(value: &serde_json::Value) -> Option<Self> {
        use super::snapshot::{unhex, word, words};
        let mut entries = VecDeque::new();
        for entry in value["entries"].as_array()? {
            let strings: Option<Vec<Vec<u8>>> = entry["strings"]
                .as_array()?
                .iter()
                .map(|text| unhex(text.as_str()?))
                .collect();
            entries.push_back(Entry {
                strings: strings?.try_into().ok()?,
                words: words(entry, "words")?.try_into().ok()?,
            });
        }
        Some(Self {
            entries,
            save_limit: word(value, "save_limit")?,
            word: value["word"].as_u64().map_or(0, |value| value as u32),
        })
    }
}

impl History {
    pub(super) fn cvns() -> Self {
        Self { word: 128, ..Self::default() }
    }

    /// CVNS 0x4216e0 prepends a three-string, two-word record and evicts
    /// the oldest entries on insertion. Changing capacity alone never evicts.
    pub(super) fn cvns_command(
        &mut self,
        machine: &mut Machine,
        opcode: u16,
    ) -> Result<u16, DecodeError> {
        let pc = machine.pc();
        let invalid = |reason| error(pc, opcode, reason);
        let arg = |i| machine.stack().peek_u32(4usize * i).map_err(|e| invalid(e.0));
        let (cleanup, result) = match opcode {
            0x2128 => (0, Some(self.entries.len() as u32)),
            0x2129..=0x212b => {
                let index = (arg(1)? as i32).max(0) as usize;
                let field = [2, 1, 0][usize::from(opcode - 0x2129)];
                let bytes = self.entries.get(index).map_or(&[][..], |entry| &entry.strings[field]);
                let dest = arg(2)?;
                machine.write_string_reference(opcode, dest, bytes)?;
                (8, None)
            }
            0x212c => {
                let mut strings = <[Vec<u8>; 4]>::default();
                for (i, string) in strings[..3].iter_mut().enumerate() {
                    let bytes = machine.resolve_string(arg(i + 1)?)?;
                    if i != 1 && bytes.len() >= 32 {
                        return Err(invalid("CVNS history label exceeds its 32-byte field"));
                    }
                    *string = bytes.to_vec();
                }
                let words = [arg(4)?, arg(5)?, 0, 0, 0, 0];
                // Native removes a tail by dereferencing its predecessor.
                if self.word == 0 || (self.word == 1 && !self.entries.is_empty()) {
                    return Err(invalid("CVNS history eviction requires a predecessor"));
                }
                self.entries.truncate((self.word - 1) as usize);
                self.entries.push_front(Entry { strings, words });
                (20, None)
            }
            0x212d => {
                self.word = arg(1)?;
                (4, None)
            }
            0x212e => (0, Some(self.word)),
            0x212f => {
                self.entries.clear();
                (0, None)
            }
            0x2136 => {
                let index = (arg(1)? as i32).max(0) as usize;
                let field = (arg(2)? & 1) as usize;
                (8, Some(self.entries.get(index).map_or(u32::MAX, |entry| entry.words[field])))
            }
            _ => unreachable!(),
        };
        if let Some(result) = result {
            machine.state_mut().set_system_integer(0, result).map_err(invalid)?;
        }
        Ok(cleanup)
    }

    #[cfg(feature = "runtime")]
    pub(super) fn csp_record(&self, out: &mut Vec<u8>) -> anyhow::Result<()> {
        let count = self.entries.len().min(self.save_limit as usize);
        let mut bytes = self.word.to_le_bytes().to_vec();
        bytes.extend_from_slice(&(count as u32).to_le_bytes());
        for entry in self.entries.iter().take(count) {
            // sub_42edf0 swaps the second and third script arguments when
            // constructing the native history node (two labels, then text).
            for index in [0, 2, 1] {
                bytes.extend_from_slice(&entry.strings[index]);
                bytes.push(0);
            }
            for word in &entry.words[..2] { bytes.extend_from_slice(&word.to_le_bytes()); }
        }
        super::save_slot::csp1::append(out, 0x540, &[], &bytes)
    }

    #[cfg(feature = "runtime")]
    pub(super) fn from_csp_record(mut bytes: &[u8], save_limit:u32) -> anyhow::Result<Self> {
        use anyhow::{Context, ensure};
        fn word(data: &mut &[u8]) -> anyhow::Result<u32> {
            let (raw,tail)=data.split_at_checked(4).context("truncated CSP1 history word")?;
            *data=tail;Ok(u32::from_le_bytes(raw.try_into()?))
        }
        let capacity=word(&mut bytes)?;
        let count=word(&mut bytes)?;
        ensure!((count as usize)<=bytes.len()/11,"truncated CSP1 history");
        let mut entries=VecDeque::new();
        for _ in 0..count {
            let mut strings=<[Vec<u8>;4]>::default();
            for index in [0,2,1] {
                let len=bytes.iter().position(|&b| b==0).context("unterminated CSP1 history string")?;
                ensure!(index==1 || len<32,"oversized CSP1 history label");
                strings[index]=bytes[..len].to_vec();bytes=&bytes[len+1..];
            }
            let words=[word(&mut bytes)?,word(&mut bytes)?,0,0,0,0];
            entries.push_back(Entry { strings, words });
        }
        ensure!(bytes.is_empty(),"trailing CSP1 history bytes");
        Ok(Self { entries, save_limit, word: capacity })
    }

    pub(super) fn clear_entries(&mut self) { self.entries.clear(); }

    #[cfg(feature="runtime")]
    pub(super) fn save_limit(&self)->u32 { self.save_limit }

    pub(super) fn set_save_limit(&mut self, limit: u32) {
        self.save_limit = limit;
    }

    pub(super) fn checkpoint(&self) -> Self {
        Self {
            entries: self
                .entries
                .iter()
                .take(self.save_limit as usize)
                .cloned()
                .collect(),
            save_limit: self.save_limit,
            word: self.word,
        }
    }

    pub(super) fn command(
        &mut self,
        machine: &mut Machine,
        opcode: u16,
    ) -> Result<u16, DecodeError> {
        let pc = machine.pc();
        let invalid = |why| error(pc, opcode, why);
        let arg = |i: usize| machine.stack().peek_u32(4 * i).map_err(|e| invalid(e.0));
        let (cleanup, result) = match opcode {
            0x212e => (0, Some(self.word)),
            0x2137 => {
                self.word = arg(1)?;
                (4, None)
            }
            0x212f => {
                self.entries.clear();
                (0, None)
            }
            0x2130 => (0, Some(self.entries.len() as u32)),
            0x2131..=0x2135 => {
                let index = (arg(1)? as i32).max(0) as usize;
                let entry = self.entries.get(index);
                let argument = arg(2)?;
                if opcode == 0x2135 {
                    (
                        8,
                        Some(entry.map_or(u32::MAX, |entry| entry.words[(argument % 6) as usize])),
                    )
                } else {
                    let StringRef::StringBank(slot) = StringRef::decode(argument) else {
                        return Err(invalid(
                            "history destination must be in the native string bank",
                        ));
                    };
                    let slot = if slot < 128 { slot } else { 0 };
                    let field = [1, 2, 0, 3][usize::from(opcode - 0x2131)];
                    let bytes = entry.map_or(&[][..], |entry| entry.strings[field].as_slice());
                    machine
                        .state_mut()
                        .set_string_journaled(slot, bytes, &mut Journal::default())
                        .map_err(invalid)?;
                    (8, None)
                }
            }
            0x2136 => {
                let mut strings: [Vec<u8>; 4] = Default::default();
                for (i, string) in strings.iter_mut().enumerate() {
                    let bytes = machine.resolve_string(arg(i + 1)?)?;
                    let limit = if i == 2 { 64 * 1024 } else { 31 };
                    if bytes.len() > limit {
                        return Err(invalid("history string exceeds its field"));
                    }
                    *string = bytes.to_vec();
                }
                let mut words = [0; 6];
                for (i, word) in words.iter_mut().enumerate() {
                    *word = arg(i + 5)?;
                }
                if self.entries.len() == CAPACITY {
                    self.entries.pop_back();
                }
                self.entries.push_front(Entry { strings, words });
                (40, None)
            }
            _ => unreachable!(),
        };
        if let Some(result) = result {
            machine
                .state_mut()
                .set_system_integer(0, result)
                .map_err(invalid)?;
        }
        Ok(cleanup)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_limit_applies_to_the_checkpoint_without_trimming_live_history() {
        let mut history = History::default();
        for index in 0..40 {
            history.entries.push_front(Entry {
                strings: Default::default(),
                words: [index; 6],
            });
        }
        let checkpoint = history.checkpoint();
        assert_eq!(checkpoint.entries.len(), 32);
        assert_eq!(checkpoint.entries.front().unwrap().words[0], 39);
        assert_eq!(checkpoint.entries.back().unwrap().words[0], 8);
        history.set_save_limit(0);
        assert!(history.checkpoint().entries.is_empty());
        history.set_save_limit(u32::MAX);
        assert_eq!(history.checkpoint().entries.len(), 40);
        assert_eq!(history.entries.len(), 40);
    }
}
