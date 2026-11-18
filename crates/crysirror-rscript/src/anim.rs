// Copyright (c) 2026 CrySirror contributors.
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. A copy is available at https://mozilla.org/MPL/2.0/.
//! Animated layers (layer kind 2, CAnim loader sub_402280): the frames are
//! the layers of `NNNN.lwg`, the flow is `NNNN.fsc` (parser sub_4034B0,
//! interpreter sub_402510, one step per tick through sub_402650).
//!
//! An `.fsc` file is one command per line (sub_44DCB0 reads a line, drops
//! a `;` comment and folds blanks): `:name` labels the position, `.` waits
//! one tick (op 2), `-` stops (op 0), `>name` jumps (op 3), `?V N name`
//! jumps when variable V equals N (op 4), `?rnd N name` jumps when
//! `rand() % N` is zero (op 5), any other line shows the frame of its
//! number (op 1, then the tick ends). Without a flow the frames advance by
//! one per tick and loop.
use crate::sprite::{Content, Point};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Anim {
    pub frames: Vec<(Content, Point)>,
    pub flow: Option<Vec<u16>>,
    pub pc: usize,
    pub frame: usize,
    pub playing: bool,
    pub looping: bool,
}

fn atoi(s: &[u8]) -> i32 {
    let s = s.iter().skip_while(|c| **c == b' ').copied().collect::<Vec<u8>>();
    let (neg, digits) = match s.first() {
        Some(b'-') => (true, &s[1..]),
        _ => (false, &s[..]),
    };
    let mut n: i32 = 0;
    for d in digits.iter().take_while(|d| d.is_ascii_digit()) {
        n = n.wrapping_mul(10).wrapping_add(i32::from(d - b'0'));
    }
    if neg { -n } else { n }
}

/// sub_4034B0: the flow words with jump targets resolved (an unknown label
/// resolves to position 0, the map's default node).
pub fn parse(src: &[u8]) -> Vec<u16> {
    let mut code: Vec<u16> = Vec::new();
    let mut labels: Vec<(Vec<u8>, u16)> = Vec::new();
    let mut fixups: Vec<(usize, Vec<u8>)> = Vec::new();
    for raw in src.split(|&b| b == b'\n') {
        let line: Vec<u8> = {
            let body = raw.split(|&b| b == b';').next().unwrap_or_default();
            let mut out = Vec::new();
            let mut blank = false;
            for &b in body.iter().filter(|&&b| b != b'\r') {
                if b == b' ' || b == b'\t' {
                    if !blank && !out.is_empty() {
                        out.push(b' ');
                    }
                    blank = true;
                } else {
                    out.push(b);
                    blank = false;
                }
            }
            while out.last() == Some(&b' ') {
                out.pop();
            }
            out
        };
        let Some(&first) = line.first() else { continue };
        let rest = &line[1..];
        let words: Vec<&[u8]> = rest.split(|&b| b == b' ').filter(|w| !w.is_empty()).collect();
        match first {
            b':' => labels.push((words.first().copied().unwrap_or_default().to_vec(), code.len() as u16)),
            b'.' => code.push(2),
            b'-' => code.push(0),
            b'>' => {
                code.push(3);
                fixups.push((code.len(), words.first().copied().unwrap_or_default().to_vec()));
                code.push(0);
            }
            b'?' => {
                let head = words.first().copied().unwrap_or_default();
                let value = words.get(1).map_or(0, |w| atoi(w));
                let label = words.get(2).copied().unwrap_or_default().to_vec();
                if head.eq_ignore_ascii_case(b"rnd") {
                    code.extend([5, value as u16]);
                } else {
                    code.extend([4, atoi(head) as u16, value as u16]);
                }
                fixups.push((code.len(), label));
                code.push(0);
            }
            _ => code.extend([1, atoi(&line) as u16]),
        }
    }
    for (at, name) in fixups {
        code[at] = labels.iter().find(|(n, _)| *n == name).map_or(0, |l| l.1);
    }
    code
}

impl Anim {
    pub fn new(frames: Vec<(Content, Point)>, flow: Option<Vec<u16>>) -> Self {
        Self { frames, flow, pc: 0, frame: 0, playing: true, looping: true }
    }
    /// sub_402510: one step; false once the animation has stopped.
    pub fn tick(&mut self, var: &dyn Fn(usize) -> i16, rand: &mut dyn FnMut() -> i32) -> bool {
        if !self.playing || self.frames.is_empty() {
            return false;
        }
        let Some(flow) = &self.flow else {
            let mut next = self.frame + 1;
            if next == self.frames.len() {
                if !self.looping {
                    self.playing = false;
                    return false;
                }
                next = 0;
            }
            self.frame = next;
            if next + 1 == self.frames.len() && !self.looping {
                self.playing = false;
            }
            return true;
        };
        let word = |pc: &mut usize| -> u16 {
            let w = flow.get(*pc).copied().unwrap_or(0);
            *pc += 1;
            w
        };
        // A flow without a frame or wait in reach ends after one pass.
        for _ in 0..flow.len() * 4 + 4 {
            match word(&mut self.pc) {
                0 => {
                    self.playing = false;
                    return false;
                }
                1 => {
                    self.frame = usize::from(word(&mut self.pc)).min(self.frames.len() - 1);
                    return true;
                }
                2 => return true,
                3 => self.pc = usize::from(word(&mut self.pc)),
                4 => {
                    let (v, n, to) = (word(&mut self.pc), word(&mut self.pc) as i16, word(&mut self.pc));
                    if var(usize::from(v)) == n {
                        self.pc = usize::from(to);
                    }
                }
                5 => {
                    let (n, to) = (i32::from(word(&mut self.pc)).max(1), word(&mut self.pc));
                    if rand() % n == 0 {
                        self.pc = usize::from(to);
                    }
                }
                _ => {}
            }
        }
        self.playing = false;
        false
    }
    pub fn current(&self) -> Option<&(Content, Point)> {
        self.frames.get(self.frame)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn flow_labels_and_ops() {
        let code = parse(b":top\r\n0 ; first\n.\n1\n?rnd 2 top\n?5 3 top\n>top\n-\n");
        assert_eq!(code, vec![1, 0, 2, 1, 1, 5, 2, 0, 4, 5, 3, 0, 3, 0, 0]);
        let frame = |n: u32| (Content::Fill { color: n, size: [1, 1] }, Point::default());
        let mut a = Anim::new(vec![frame(0), frame(1)], Some(code));
        let mut r = || 1;
        assert!(a.tick(&|_| 0, &mut r));
        assert_eq!(a.frame, 0);
        assert!(a.tick(&|_| 0, &mut r)); // wait
        assert!(a.tick(&|_| 0, &mut r));
        assert_eq!(a.frame, 1);
        // rnd misses, var 5 == 3 hits: back to the top.
        assert!(a.tick(&|v| if v == 5 { 3 } else { 0 }, &mut r));
        assert_eq!((a.frame, a.pc), (0, 2));
    }
}
