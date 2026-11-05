// SPDX-License-Identifier: MPL-2.0
//! Rational frame admission using a caller-owned wrapping millisecond clock.
//! Late polling admits one update and catches up the phase without replaying
//! missed updates. The caller retains ownership of input and pending work.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FrameAdmissionClock {
    epoch: u32,
    phase: u32,
    period_ms: u32,
    subdivisions: u32,
}
impl FrameAdmissionClock {
    /// The first update is due at `epoch + period_ms / subdivisions`.
    pub fn new(epoch: u32, period_ms: u32, subdivisions: u32) -> crate::Result<Self> {
        if period_ms == 0 || period_ms > i32::MAX as u32
            || subdivisions == 0 || subdivisions > period_ms
        {
            return Err(crate::Error::Invalid("frame admission period"));
        }
        Ok(Self { epoch, phase: 0, period_ms, subdivisions })
    }
    /// `force` admits work between scheduled updates without moving the phase.
    /// As with a wrapping native clock, polls must be less than 2^31 ms apart.
    pub fn poll(&mut self, now: u32, force: bool) -> bool {
        let elapsed = now.wrapping_sub(self.epoch) as i32;
        if elapsed < 0 { return force; }
        let elapsed = elapsed as u32;
        let cycles = elapsed / self.period_ms;
        let remainder = elapsed % self.period_ms;
        let phase = (((u64::from(remainder) + 1) * u64::from(self.subdivisions) - 1)
            / u64::from(self.period_ms)) as u32;
        let due = cycles != 0 || phase > self.phase;
        self.epoch = self.epoch.wrapping_add(cycles * self.period_ms);
        self.phase = phase;
        force || due
    }
    /// Deadline in the caller's nonwrapping clock domain.
    pub fn deadline(&self, now_ms: u64) -> u64 {
        let offset = (u64::from(self.period_ms) * u64::from(self.phase + 1)
            / u64::from(self.subdivisions)) as u32;
        let remaining = self.epoch.wrapping_add(offset).wrapping_sub(now_ms as u32) as i32;
        now_ms.saturating_add(remaining.max(0) as u64)
    }
}
