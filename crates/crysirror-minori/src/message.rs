// SPDX-License-Identifier: MPL-2.0
//! Native escape and input timing. CrySirror lays out and draws the resulting text.
use super::message_action::MessageAction;
use anyhow::{Result, bail};
/// Name used for ANI lookup. The panel retains the original display name.
pub(super) fn speaker_id(raw: &str) -> Option<String> {
    if raw.is_empty() {
        return Some(String::new());
    }
    let name = raw.strip_prefix(['@', '#']).unwrap_or(raw);
    let (bytes, _, _) = encoding_rs::SHIFT_JIS.encode(name);
    let mut bytes = bytes.into_owned();
    for pair in bytes.chunks_exact_mut(2) {
        if pair == [0x81, 0x40] {
            pair.fill(b' ');
        }
    }
    bytes.retain(|b| *b != b' ');
    if bytes.is_empty() {
        return None;
    }
    let bytes = bytes.split(|b| *b == b'=').next().unwrap_or_default();
    Some(encoding_rs::SHIFT_JIS.decode(bytes).0.into_owned())
}

enum Gate {
    Voice,
    Delay,
    Action(MessageAction),
}
pub(super) fn display_name(speaker: &str) -> &str {
    if speaker.starts_with('#') { return ""; }
    if speaker.starts_with('@') { return "？？？"; }
    speaker.split_once('=').map_or(speaker, |(_, name)| name)
}
pub(super) struct Message {
    gates: std::collections::VecDeque<(u64, Gate)>,
    blocked_since: Option<u64>,
    pub auto_advance: bool,
    text: String,
    ends: Vec<(u64, usize)>,
    ticks: u64,
    start: u64,
    interval_ms: u64,
    shown: Option<usize>,
}
impl Message {
    pub fn new(speaker: &str, body: &str, start: u64, speed: u8) -> Result<Self> {
        let mut text = String::new();
        let name = display_name(speaker);
        if !name.is_empty() {
            text = format!("【{name}】\n")
        }
        let mut ends = vec![(0, text.len())];
        let mut ticks = 0;
        let mut gates = std::collections::VecDeque::new();
        let mut auto_advance = false;
        let mut chars = body.chars();
        while let Some(mut ch) = chars.next() {
            if ch == '\\' {
                match chars.next() {
                    Some('a') => {
                        auto_advance = true;
                        continue;
                    }
                    Some(control @ ('v' | 'w')) => {
                        if control == 'w' {
                            auto_advance = false;
                        }
                        gates.push_back((
                            ticks,
                            if control == 'v' {
                                Gate::Voice
                            } else {
                                Gate::Delay
                            },
                        ));
                        continue;
                    }
                    Some('x') => {
                        if chars.next() != Some('{') {
                            bail!("minori inline command requires an opening brace");
                        }
                        let mut command = String::new();
                        for part in chars.by_ref() {
                            if part == '}' {
                                gates.push_back((
                                    ticks,
                                    Gate::Action(MessageAction::parse(&command, 0)?),
                                ));
                                break;
                            }
                            command.push(part);
                        }
                        continue;
                    }
                    Some('n') => ch = '\n',
                    Some('N') => {
                        text.push('\n');
                        ends.push((ticks, text.len()));
                        continue;
                    }
                    Some(control) => bail!("unsupported minori message escape \\{control}"),
                    None => bail!("unterminated minori message escape"),
                }
            }
            ticks += 1;
            text.push(ch);
            ends.push((ticks, text.len()));
        }
        Ok(Self {
            text,
            ends,
            gates,
            blocked_since: None,
            auto_advance,
            ticks,
            start,
            interval_ms: u64::from(speed / 5) * 10,
            shown: None,
        })
    }
    pub fn frame(
        &mut self,
        now: u64,
        finish: bool,
        voice_playing: bool,
        actions: &mut Vec<MessageAction>,
    ) -> (Option<&str>, bool) {
        let mut progress = if finish || self.interval_ms == 0 {
            self.ticks
        } else {
            1 + now.saturating_sub(self.start) / self.interval_ms
        };
        while let Some((at, gate)) = self.gates.front() {
            if *at > progress {
                break;
            }
            if matches!(gate, Gate::Action(_)) {
                if let Some((_, Gate::Action(mut action))) = self.gates.pop_front() {
                    action.due = action.due.wrapping_add(now / 10);
                    actions.push(action);
                }
                continue;
            }
            let since = *self.blocked_since.get_or_insert(now);
            let waiting = match gate {
                Gate::Voice => voice_playing,
                Gate::Delay => now.saturating_sub(since) < 100,
                Gate::Action(_) => unreachable!(),
            };
            if waiting && !finish {
                progress = *at;
                break;
            }
            self.start = self.start.saturating_add(now.saturating_sub(since));
            self.blocked_since = None;
            self.gates.pop_front();
            if !finish && self.interval_ms != 0 {
                progress = 1 + now.saturating_sub(self.start) / self.interval_ms;
            }
        }
        let count = self.ends.partition_point(|(tick, _)| *tick <= progress);
        let end = self.ends[count - 1].1;
        let changed = self.shown.replace(end) != Some(end);
        (
            changed.then_some(&self.text[..end]),
            progress >= self.ticks && self.gates.is_empty(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inline_delay_starts_when_the_escape_is_read() {
        let mut message = Message::new("", r"AB\x{vis,3,7,1}C", 0, 50).unwrap();
        let mut actions = Vec::new();
        assert_eq!(message.frame(0, false, false, &mut actions).0, Some("A"));
        assert!(actions.is_empty());
        assert_eq!(message.frame(100, false, false, &mut actions).0, Some("AB"));
        assert_eq!(actions[0].due, 13);
        assert_eq!(actions[0].id, 7);
    }

    #[test]
    fn voice_gate_resumes_the_glyph_clock_and_auto_advance_survives() {
        let mut m = Message::new("", r"A\vB\a", 0, 50).unwrap();
        assert_eq!(m.frame(0, false, true, &mut Vec::new()), (Some("A"), false));
        assert_eq!(m.frame(1000, false, true, &mut Vec::new()), (None, false));
        assert_eq!(m.frame(1001, false, false, &mut Vec::new()), (None, false));
        assert_eq!(
            m.frame(1101, false, false, &mut Vec::new()),
            (Some("AB"), true)
        );
        assert!(m.auto_advance);
        let mut m = Message::new("", r"A\a\wB", 0, 0).unwrap();
        assert_eq!(
            m.frame(0, false, false, &mut Vec::new()),
            (Some("A"), false)
        );
        assert_eq!(m.frame(99, false, false, &mut Vec::new()), (None, false));
        assert_eq!(
            m.frame(100, false, false, &mut Vec::new()),
            (Some("AB"), true)
        );
        assert!(!m.auto_advance);
    }
}
