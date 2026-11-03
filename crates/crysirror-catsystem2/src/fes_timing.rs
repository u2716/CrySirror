// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Original7563A0/77A070/77A830 frame and numeric-wait policy.
//! Object/string/privateflag waits require their real resource observations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Wait {
    Until(u32),
    Frame(u32),
    /// Native-1 wait; actual engine-owned object motion must be observed by
    /// the semantic driver. Clearing it still yields this native owner tick.
    Objects,
    /// 77A070 result -4: until the named object (a SOUND's playback) has finished.
    Object,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct State {
    pub base: u32,
    pub now: u32,
    pub wait: Option<Wait>,
}
impl State {
    pub fn observe(&mut self, clock: u32, skip_enabled: bool, click_down: bool) -> bool {
        if self.base == 0 {
            self.base = clock.wrapping_sub(1);
        }
        self.now = clock;
        let ready = match self.wait {
            None => true,
            //The reviewed binary really uses unsigned absolute comparison
            //for numeric delays; do not repair its wrap behavior silently.
            Some(Wait::Until(deadline)) => clock >= deadline || (skip_enabled && click_down),
            Some(Wait::Frame(target)) => self.frame() >= target || (skip_enabled && click_down),
            Some(Wait::Objects) => false,
            Some(Wait::Object) => skip_enabled && click_down,
        };
        if ready {
            self.wait = None;
        }
        ready
    }
    pub fn frame(&self) -> u32 {
        self.now.wrapping_sub(self.base)
    }
    pub fn numeric_wait(&mut self, duration: u32) {
        let duration = if duration as i32 <= 0 { 1 } else { duration };
        self.wait = Some(Wait::Until(self.now.wrapping_add(duration)));
    }
    pub fn frame_wait(&mut self, target: u32) {
        self.wait = Some(Wait::Frame(target));
    }
    pub fn reset_frame(&mut self) {
        self.base = 0;
        self.now = 0;
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_numeric_wait_observes_deadline_and_actual_skip_state() {
        let mut s = State::default();
        assert!(s.observe(100, false, false));
        assert_eq!(s.frame(), 1);
        s.numeric_wait(3);
        assert!(!s.observe(100, false, true));
        assert!(!s.observe(102, false, false));
        assert!(s.observe(103, false, false));
        s.numeric_wait(0);
        assert!(!s.observe(103, false, false));
        assert!(s.observe(104, false, false));
        s.numeric_wait(u32::MAX);
        assert!(!s.observe(104, false, false));
        assert!(s.observe(104, true, true));
    }
    #[test]
    fn native_relative_frame_wait_and_reset_preserve_first_tick_one() {
        let mut s = State::default();
        s.observe(10, false, false);
        s.frame_wait(5);
        assert!(!s.observe(13, false, false));
        assert!(s.observe(14, false, false));
        s.reset_frame();
        assert_eq!(s.frame(), 0);
        s.observe(80, false, false);
        assert_eq!(s.frame(), 1);
        s.frame_wait(80);
        assert!(s.observe(81, true, true));
        let mut wrap = State::default();
        wrap.observe(u32::MAX, false, false);
        wrap.numeric_wait(2);
        assert!(wrap.observe(u32::MAX, false, false));
    }
}
