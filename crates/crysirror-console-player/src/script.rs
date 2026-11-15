// SPDX-License-Identifier: MPL-2.0
//! Unattended test input: `<root>/input-script`, one event per line.
//!   `<frame> button <index>`   press for four frames (shared console indices)
//!   `<frame> tap <x> <y>`      touch for four frames at display pixels
//!   `<frame> hold <index>`     press until a matching `release`
//!   `<frame> release <index>`
//! Lines starting with `#` are comments. Absent file: no events.
use std::path::Path;

#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    Button(u8),
    Tap(f64, f64),
    Hold(u8, bool),
}

#[derive(Default)]
pub struct Script(Vec<(u64, Event)>);

pub enum Action {
    Button(u8, bool),
    Touch(i32, f64, f64),
}

impl Script {
    pub fn load(root: &Path) -> Self {
        Self::parse(&std::fs::read_to_string(root.join("input-script")).unwrap_or_default())
    }
    pub fn parse(text: &str) -> Self {
        let mut events = Vec::new();
        for line in text.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with('#')) {
            let p: Vec<&str> = line.split_whitespace().collect();
            let Some(frame) = p.first().and_then(|f| f.parse().ok()) else { continue };
            match p.get(1).copied() {
                Some("button") => {
                    if let Some(b) = p.get(2).and_then(|b| b.parse().ok()) {
                        events.push((frame, Event::Button(b)));
                    }
                }
                Some(kind @ ("hold" | "release")) => {
                    if let Some(b) = p.get(2).and_then(|b| b.parse().ok()) {
                        events.push((frame, Event::Hold(b, kind == "hold")));
                    }
                }
                Some("tap") => {
                    if let (Some(x), Some(y)) = (p.get(2).and_then(|v| v.parse().ok()), p.get(3).and_then(|v| v.parse().ok())) {
                        events.push((frame, Event::Tap(x, y)));
                    }
                }
                _ => {}
            }
        }
        Self(events)
    }
    /// Actions due on `frame`.
    pub fn actions(&self, frame: u64) -> Vec<Action> {
        let mut out = Vec::new();
        for (at, event) in &self.0 {
            if !(*at..=at + 4).contains(&frame) {
                continue;
            }
            let k = frame - at;
            match event {
                Event::Hold(b, down) if k == 0 => out.push(Action::Button(*b, *down)),
                Event::Button(b) if k == 0 => out.push(Action::Button(*b, true)),
                Event::Button(b) if k == 4 => out.push(Action::Button(*b, false)),
                Event::Tap(x, y) => out.push(Action::Touch(if k == 0 { 0 } else if k == 4 { 2 } else { 1 }, *x, *y)),
                _ => {}
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parse_and_schedule() {
        let s = Script::parse("# c\n10 button 0\n20 tap 1.5 2\nbad\n");
        assert!(matches!(s.actions(10)[..], [Action::Button(0, true)]));
        assert!(matches!(s.actions(14)[..], [Action::Button(0, false)]));
        assert!(s.actions(12).is_empty());
        assert!(matches!(s.actions(22)[..], [Action::Touch(1, x, y)] if x == 1.5 && y == 2.0));
    }
    #[test]
    fn hold_lasts_until_release() {
        let s = Script::parse("5 hold 7
500 release 7
");
        assert!(matches!(s.actions(5)[..], [Action::Button(7, true)]));
        assert!(s.actions(9).is_empty() && s.actions(100).is_empty());
        assert!(matches!(s.actions(500)[..], [Action::Button(7, false)]));
    }
}
