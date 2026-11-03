// SPDX-License-Identifier: MPL-2.0
//! Reusable inactivity deadlines on an owner-supplied clock and activity token.
//! No device, input sampling, operating-system timer or VM dependency.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActivityTimer {
    pub last_activity_ms: u64,
    pub sequence: Option<u64>,
}
impl ActivityTimer {
    pub fn new(now_ms: u64, sequence: Option<u64>) -> Self {
        Self {
            last_activity_ms: now_ms,
            sequence,
        }
    }
    /// None preserves unavailable metadata. A token change is one observation,
    /// not every simulation/input revision, and restarts the inactivity period.
    pub fn observe(&mut self, now_ms: u64, sequence: Option<u64>) {
        if let Some(sequence) = sequence {
            if self.sequence != Some(sequence) {
                self.last_activity_ms = now_ms;
                self.sequence = Some(sequence);
            }
        }
    }
    pub fn inactive_for(&self, now_ms: u64, delay_ms: u64) -> bool {
        now_ms >= self.last_activity_ms && now_ms - self.last_activity_ms >= delay_ms
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn explicit_activity_not_frame_sampling_restarts_exact_deadline() {
        let mut t = ActivityTimer::new(10, Some(5));
        t.observe(109, Some(5));
        assert!(!t.inactive_for(109, 100));
        assert!(t.inactive_for(110, 100));
        t.observe(111, Some(6));
        assert!(!t.inactive_for(210, 100));
        assert!(t.inactive_for(211, 100));
        t.observe(212, None);
        assert!(t.inactive_for(212, 100));
    }
    #[test]
    fn saved_data_unknown_tokens_and_full_clock_domain_remain_defined() {
        let mut t = ActivityTimer::new(100, None);
        assert!(!t.inactive_for(99, 0));
        t.observe(101, None);
        assert!(t.inactive_for(200, 100));
        t.observe(201, Some(u64::MAX));
        assert!(!t.inactive_for(201, 1));
        let restored: ActivityTimer =
            serde_json::from_slice(&serde_json::to_vec(&t).unwrap()).unwrap();
        assert_eq!(restored, t);
        assert!(restored.inactive_for(u64::MAX, u64::MAX - 201));
    }
}
