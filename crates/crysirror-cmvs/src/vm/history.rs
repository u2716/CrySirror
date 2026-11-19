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
