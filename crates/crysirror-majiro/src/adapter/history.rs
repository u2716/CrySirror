// Copyright (c) 2026 CrySirror contributors. MIT.
//! Message history (40000-byte record ring of 417070/4156E0) and its page
//! walkers (416B50, 4169F0) and replay (416280): records are redrawn through
//! the console without hooks or new history.
use super::*;

const HISTORY: usize = 40000;
pub(super) struct Replay {
    ctx: u64,
    pos: usize,
    first: bool,
    paused: bool,
    text: bool,
}
/// One step of the shared page-start walk; `start` is a record end offset.
fn page_start(h: &[u8], start: usize, pause_breaks: bool) -> usize {
    let size_at = |end: usize| -> usize {
        if end < 2 || end > h.len() { 0 } else { u16::from_le_bytes([h[end - 2], h[end - 1]]) as usize }
    };
    let mut pos = start;
    let mut seen = false;
    loop {
        let size = size_at(pos);
        if size == 0 {
            return pos;
        }
        if pos <= size + 2 {
            return start;
        }
        pos -= size;
        let b = h[pos];
        if b == b'\\' {
            if h.get(pos + 1) == Some(&b'p') {
                seen = true;
            }
        } else {
            seen = true;
        }
        let clear = b == b'\\' && h.get(pos + 1) == Some(&b'w');
        let pause_newline = !pause_breaks
            && pos >= 5
            && b == b'\\'
            && h.get(pos + 1) == Some(&b'n')
            && h[pos - 5] == b'\\'
            && h[pos - 4] == b'p';
        if !(pos < HISTORY && !clear && !pause_newline) && seen {
            return pos;
        }
    }
}
impl MajiroVm {
    pub(super) fn history_page_start(&self, from: i32) -> i32 {
        let start = if from < 0 { HISTORY } else { (from as usize).min(HISTORY) };
        page_start(self.console.history(), start, false) as i32
    }
    pub(super) fn history_pages_back(&self, pages: i32) -> i32 {
        let h = self.console.history();
        let mut pos = page_start(h, HISTORY, false);
        for _ in 1..pages.max(1) {
            pos = page_start(h, pos, false);
        }
        pos as i32
    }
    /// 307F28BC/416280: one native replay execution over the history from
    /// `from`. Controls run through 4156E0 with recording off (hooks still
    /// fire); text runs PROC in replay mode. The pending call is kept (PC
    /// rewind) until the page ends; then it completes with 0.
    pub(super) fn replay_step(&mut self, m: &mut Machine, from: i32, returns: bool, host: &mut dyn HostServices) -> Result<bool> {
        let ctx = m.context_id();
        if self.replay.as_ref().map(|r| r.ctx) != Some(ctx) {
            let pos = if from < 0 { self.history_page_start(-1) as usize } else { (from as usize).min(HISTORY) };
            self.replay = Some(Replay { ctx, pos, first: true, paused: false, text: false });
            self.text_buffer.clear();
        }
        self.console.replay = true;
        loop {
            if self.replay.as_ref().unwrap().text {
                match self.proc_line(m, host, true)? {
                    text::ProcStep::Wait => return Ok(false),
                    text::ProcStep::Hooked => return Ok(true),
                    text::ProcStep::Line => continue,
                    text::ProcStep::Empty => self.replay.as_mut().unwrap().text = false,
                }
            }
            let (pos, first, paused) = {
                let r = self.replay.as_ref().unwrap();
                (r.pos, r.first, r.paused)
            };
            let h = self.console.history();
            let record = if pos < HISTORY - 2 {
                h[pos..HISTORY].iter().position(|b| *b == 0).map(|p| h[pos..pos + p].to_vec())
            } else {
                None
            };
            let Some(record) = record else {
                return self.finish_replay(m, returns);
            };
            let next = pos + record.len() + 3;
            if record.first() == Some(&b'\\') {
                let tag = record.get(1).copied().unwrap_or(0);
                if !first && tag == b'w' {
                    return self.finish_replay(m, returns);
                }
                if !first && paused && tag == b'n' {
                    return self.finish_replay(m, returns);
                }
                {
                    let r = self.replay.as_mut().unwrap();
                    r.pos = next;
                    r.first = false;
                }
                let mut tokens: Vec<Vec<u8>> = vec![];
                match tag {
                    b'p' => self.replay.as_mut().unwrap().paused = true,
                    b'c' | b'v' => {}
                    b'n' if first => tokens.push(b"w".to_vec()),
                    _ => {
                        tokens.push(record[1..].to_vec());
                        if first && tag != b'w' {
                            tokens.push(b"w".to_vec());
                        }
                    }
                }
                let mut hooked = false;
                for token in tokens {
                    let mut candidate = self.console.clone();
                    let callback = self.console_token(&mut candidate, &token, host)?;
                    self.console = candidate;
                    if let Some((event, args)) = callback {
                        hooked |= m.interrupt_with_event(event, args)?;
                    }
                }
                if hooked {
                    return Ok(true);
                }
            } else {
                let r = self.replay.as_mut().unwrap();
                r.pos = next;
                r.first = false;
                r.text = !record.is_empty();
                self.text_buffer = record;
            }
        }
    }
    fn finish_replay(&mut self, m: &mut Machine, returns: bool) -> Result<bool> {
        self.replay = None;
        self.console.replay = false;
        self.text_buffer.clear();
        m.acknowledge(if returns { vec![Value::Integer(0)] } else { vec![] })?;
        Ok(true)
    }
}
