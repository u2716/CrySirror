// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Retained reading deadlines over caller-owned logical time. Extracted from
//! siglus_rs5f358ef runtime/ui.rs::auto_advance_due; native override/config and
//! message-proc policy remain in the adapter. No host clock or input is read.
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReadingWait {
    pub started_at_ms: u64,
    pub characters: usize,
}
impl ReadingWait {
    pub fn due(&self, now_ms: u64, milliseconds_per_character: u64, minimum_ms: u64) -> bool {
        let characters = u64::try_from(self.characters.max(1)).unwrap_or(u64::MAX);
        let duration = characters
            .saturating_mul(milliseconds_per_character)
            .max(minimum_ms);
        now_ms.saturating_sub(self.started_at_ms) >= duration
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fixed_reading_deadline_survives_restore_and_keeps_minimum_and_clock_boundaries() {
        let wait = ReadingWait {
            started_at_ms: 100,
            characters: 3,
        };
        assert!(!wait.due(399, 80, 300));
        assert!(wait.due(400, 80, 300));
        let restored: ReadingWait =
            serde_json::from_slice(&serde_json::to_vec(&wait).unwrap()).unwrap();
        assert!(!restored.due(339, 80, 0));
        assert!(restored.due(340, 80, 0));
        assert!(!restored.due(50, 80, 0));
        let empty = ReadingWait {
            started_at_ms: 0,
            characters: 0,
        };
        assert!(!empty.due(79, 80, 0));
        assert!(empty.due(80, 80, 0));
        let large = ReadingWait {
            started_at_ms: 0,
            characters: usize::MAX,
        };
        assert!(!large.due(u64::MAX - 1, u64::MAX, 0));
        assert!(large.due(u64::MAX, u64::MAX, 0));
    }
}
