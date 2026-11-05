// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Bounded adaptive fixed-step debt, independent of devices, VMs and wall time.
//! Hosts open one update, consume its planned work, then close it. A host that
//! does no fixed-step work drops old debt instead of accumulating a later burst.
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

/// Serializable adaptive step scheduling state. All methods validate restored
/// state before use; malformed or overflowing updates leave the state unchanged.
/// Configuration follows signed32 finite arithmetic, rejecting its overflow
/// domain rather than relying on a platform's unchecked integer behaviour.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdaptiveStepClock {
    /// Positive fixed-step duration in milliseconds.
    pub period_ms: i32,
    /// Maximum planned work per owner update.
    pub maximum_steps: i32,
    /// Positive response horizon measured in fixed steps.
    pub horizon: i32,
    /// Explicit prediction lead; zero reacts to existing debt only.
    pub lookahead_steps: i32,
    /// Signed unconsumed time; overpayment is bounded below.
    pub debt_ms: i32,
    /// Last adaptive work plan, bounded by maximum_steps.
    pub planned_steps: i32,
    /// Whether this open update performed even a zero consumption.
    pub consumed: bool,
    /// Whether begin has opened an update that still requires end.
    pub open: bool,
}
impl AdaptiveStepClock {
    /// Positive period (1..60000ms), work cap (1..1024) and response horizon
    /// (1..10000). Their product must fit signed32 arithmetic.
    pub fn new(period_ms: u32, maximum_steps: u32, horizon: u32) -> Result<Self> {
        Self::with_lookahead(period_ms, maximum_steps, horizon, 0)
    }
    /// Explicit prediction lead measured in fixed periods (0..10000). A zero
    /// lead reacts to existing debt; a positive lead may schedule work ahead.
    pub fn with_lookahead(
        period_ms: u32,
        maximum_steps: u32,
        horizon: u32,
        lookahead_steps: u32,
    ) -> Result<Self> {
        let result = Self {
            lookahead_steps: lookahead_steps.try_into()?,
            period_ms: period_ms.try_into()?,
            maximum_steps: maximum_steps.try_into()?,
            horizon: horizon.try_into()?,
            debt_ms: 0,
            planned_steps: 0,
            consumed: true,
            open: false,
        };
        result.validate()?;
        Ok(result)
    }
    /// Check the complete serializable state, including the signed debt domain.
    pub fn validate(&self) -> Result<()> {
        ensure!(
            (1..=60000).contains(&self.period_ms)
                && (1..=1024).contains(&self.maximum_steps)
                && (1..=10000).contains(&self.horizon)
                && (0..=10000).contains(&self.lookahead_steps),
            "adaptive step configuration"
        );
        let horizon_ms = self
            .period_ms
            .checked_mul(self.horizon)
            .ok_or_else(|| anyhow::anyhow!("adaptive step horizon overflow"))?;
        ensure!(
            horizon_ms.checked_mul(self.maximum_steps).is_some(),
            "adaptive step work overflow"
        );
        let lookahead_ms = self
            .period_ms
            .checked_mul(self.lookahead_steps)
            .ok_or_else(|| anyhow::anyhow!("adaptive step lookahead overflow"))?;
        ensure!(
            (-60000..=i32::MAX - lookahead_ms).contains(&self.debt_ms)
                && (0..=self.maximum_steps).contains(&self.planned_steps),
            "adaptive step state domain"
        );
        Ok(())
    }
    /// Change period without inventing an elapsed update or discarding debt.
    pub fn set_period(&mut self, period_ms: u32) -> Result<()> {
        let mut candidate = self.clone();
        candidate.period_ms = period_ms.try_into()?;
        candidate.validate()?;
        *self = candidate;
        Ok(())
    }
    /// Open exactly one host update; elapsed time is supplied by its owner.
    /// The planned step count changes by at most one in either direction.
    pub fn begin(&mut self, elapsed_ms: u32) -> Result<()> {
        self.validate()?;
        ensure!(!self.open, "adaptive step update already open");
        let mut candidate = self.clone();
        candidate.debt_ms = candidate
            .debt_ms
            .checked_add(elapsed_ms.try_into()?)
            .ok_or_else(|| anyhow::anyhow!("adaptive step debt overflow"))?;
        candidate.consumed = false;
        candidate.open = true;
        candidate.validate()?;
        if candidate.debt_ms < candidate.period_ms {
            candidate.planned_steps = 0;
        } else {
            let horizon_ms = candidate.horizon * candidate.period_ms;
            let threshold = candidate.debt_ms + candidate.lookahead_steps * candidate.period_ms;
            if candidate.planned_steps * horizon_ms < threshold {
                candidate.planned_steps += 1;
            }
            if candidate.planned_steps > 1 && (candidate.planned_steps - 1) * horizon_ms > threshold
            {
                candidate.planned_steps -= 1;
            }
            candidate.planned_steps = candidate.planned_steps.min(candidate.maximum_steps);
        }
        candidate.validate()?;
        *self = candidate;
        Ok(())
    }
    /// Planned work for the open update, optionally limited by the consumer.
    /// This read does not consume debt or mark an update as processed.
    pub fn available(&self, limit: Option<u32>) -> Result<u32> {
        self.validate()?;
        ensure!(self.open, "adaptive step update is not open");
        let planned = self.planned_steps as u32;
        Ok(limit.map_or(planned, |cap| planned.min(cap)))
    }
    /// Consume fixed steps; zero is meaningful and marks the update processed.
    /// A consumer may overpay by at most one period after clamping.
    pub fn consume_frames(&mut self, frames: u32) -> Result<()> {
        self.validate()?;
        ensure!(self.open, "adaptive step update is not open");
        let millis = i32::try_from(frames)?
            .checked_mul(self.period_ms)
            .ok_or_else(|| anyhow::anyhow!("adaptive step consumption overflow"))?;
        self.consume_time(millis as u32)
    }
    /// Consume a time amount in the same bounded debt domain.
    pub fn consume_time(&mut self, elapsed_ms: u32) -> Result<()> {
        self.validate()?;
        ensure!(self.open, "adaptive step update is not open");
        let debt = self
            .debt_ms
            .checked_sub(elapsed_ms.try_into()?)
            .ok_or_else(|| anyhow::anyhow!("adaptive step repayment overflow"))?;
        self.debt_ms = debt.max(-self.period_ms);
        self.consumed = true;
        Ok(())
    }
    /// Close an update. If no consumer ran, clear both debt and work planning.
    pub fn end(&mut self) -> Result<()> {
        self.validate()?;
        ensure!(self.open, "adaptive step update is not open");
        if !self.consumed {
            self.debt_ms = 0;
            self.planned_steps = 0;
            self.consumed = true;
        }
        self.open = false;
        Ok(())
    }
    /// Clear scheduling debt without advancing the host clock.
    pub fn clear(&mut self) -> Result<()> {
        self.validate()?;
        self.debt_ms = 0;
        self.planned_steps = 0;
        self.consumed = true;
        Ok(())
    }
    pub fn debt_ms(&self) -> i32 {
        self.debt_ms
    }
    pub fn planned_steps(&self) -> i32 {
        self.planned_steps
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn irregular_updates_and_consumer_caps_follow_adaptive_debt_not_elapsed_floor() {
        let mut c = AdaptiveStepClock::with_lookahead(50, 3, 10, 10).unwrap();
        for (elapsed, cap, expected_plan, expected_debt) in [
            (0, 0, 0, 0),
            (50, 1, 1, 0),
            (500, 1, 2, 450),
            (100, 3, 3, 400),
            (0, 0, 2, 400),
        ] {
            c.begin(elapsed).unwrap();
            assert_eq!(c.planned_steps(), expected_plan);
            let steps = c.available(Some(cap)).unwrap();
            c.consume_frames(steps).unwrap();
            c.end().unwrap();
            assert_eq!(c.debt_ms(), expected_debt);
        }
        c.begin(0).unwrap();
        assert_eq!(c.available(None).unwrap(), 2);
        c.end().unwrap();
        assert_eq!((c.debt_ms(), c.planned_steps()), (0, 0));
    }
    #[test]
    fn overpayment_zero_consumption_period_change_and_roundtrip_preserve_real_state() {
        let mut c = AdaptiveStepClock::new(50, 3, 10).unwrap();
        c.begin(0).unwrap();
        c.consume_time(100).unwrap();
        c.end().unwrap();
        assert_eq!(c.debt_ms(), -50);
        c.set_period(25).unwrap();
        c.begin(0).unwrap();
        c.consume_frames(0).unwrap();
        c.end().unwrap();
        assert_eq!(c.debt_ms(), -25);
        c.begin(24).unwrap();
        c.consume_frames(0).unwrap();
        c.end().unwrap();
        assert_eq!(c.debt_ms(), -1);
        c.begin(26).unwrap();
        assert_eq!(c.available(None).unwrap(), 1);
        let mut restored: AdaptiveStepClock =
            serde_json::from_slice(&serde_json::to_vec(&c).unwrap()).unwrap();
        restored.consume_frames(1).unwrap();
        restored.end().unwrap();
        c.consume_frames(1).unwrap();
        c.end().unwrap();
        assert_eq!(c, restored);
    }
    #[test]
    fn invalid_restored_state_and_signed32_overflow_are_rejected_before_mutation() {
        assert!(AdaptiveStepClock::new(60000, 1024, 10000).is_err());
        let mut c = AdaptiveStepClock::new(50, 3, 10).unwrap();
        let before = c.clone();
        assert!(c.begin(u32::MAX).is_err());
        assert_eq!(c, before);
        c.begin(0).unwrap();
        let before = c.clone();
        assert!(c.begin(1).is_err());
        assert!(c.consume_frames(u32::MAX).is_err());
        assert_eq!(c, before);
        let mut invalid = c.clone();
        invalid.planned_steps = 4;
        assert!(invalid.available(None).is_err());
        assert!(invalid.end().is_err());
        c.end().unwrap();
        assert!(c.available(None).is_err());
        assert!(c.consume_frames(1).is_err());
    }
}

#[cfg(test)]
mod zero_lead_tests {
    use super::*;
    #[test]
    fn zero_lead_profile_matches_actual_legacy_debt_comparison_not_later_prediction() {
        let mut c = AdaptiveStepClock::new(50, 3, 10).unwrap();
        for (elapsed, cap, plan, debt) in [
            (50, 1, 1, 0),
            (500, 1, 1, 450),
            (100, 3, 2, 450),
            (0, 0, 1, 450),
        ] {
            c.begin(elapsed).unwrap();
            assert_eq!(c.planned_steps(), plan);
            c.consume_frames(c.available(Some(cap)).unwrap()).unwrap();
            c.end().unwrap();
            assert_eq!(c.debt_ms(), debt);
        }
        let mut ahead = AdaptiveStepClock::with_lookahead(50, 3, 10, 10).unwrap();
        ahead.begin(500).unwrap();
        ahead.consume_frames(1).unwrap();
        ahead.end().unwrap();
        ahead.begin(0).unwrap();
        assert_eq!(ahead.planned_steps(), 2);
        // The caller must explicitly choose prediction; it is not inferred
        // from a native VM name or silently substituted during restoration.
        assert_ne!(ahead.available(None).unwrap(), c.planned_steps() as u32);
    }
}
