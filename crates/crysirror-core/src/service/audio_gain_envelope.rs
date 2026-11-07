//! Mixer-owned, output-clock stepped attenuation. Values are integer dB100;
//! -10000 is the explicit legacy silence sentinel, 0 is unity amplitude.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AudioGainEnvelopeSpec {
    pub interval_ms: u32,
    pub steps: u32,
    pub start_db100: i32,
    pub end_db100: i32,
    /// Resume a partially completed envelope without reconstructing its PCM.
    #[serde(default)]
    pub elapsed_steps: u32,
    /// Millisecond remainder already accumulated toward the next step.
    #[serde(default)]
    pub phase_ms: u32,
    /// Apply the initial indexed level immediately, or retain the source's
    /// present amplitude until its first scheduled update.
    #[serde(default = "apply_initial")]
    pub apply_initial: bool,
    #[serde(default)]
    pub stop_at_end: bool,
}
fn apply_initial() -> bool {
    true
}
impl AudioGainEnvelopeSpec {
    pub fn validate(self) -> crate::Result<()> {
        if self.interval_ms == 0
            || self.interval_ms > 60_000
            || self.steps == 0
            || self.steps > 1_000_000
            || self.elapsed_steps > self.steps
            || self.phase_ms >= self.interval_ms
            || !(-10000..=0).contains(&self.start_db100)
            || !(-10000..=0).contains(&self.end_db100)
        {
            return Err(crate::Error::Invalid("stepped audio gain envelope"));
        }
        Ok(())
    }
    /// Weighted nonnegative attenuation levels are floored before restoring
    /// their -10000 origin. Decreasing ramps therefore match the increasing
    /// ramp's exact complementary weights rather than signed-division bias.
    pub fn db100_at(self, step: u32) -> i32 {
        let count = u64::from(step.min(self.steps));
        let total = u64::from(self.steps.max(1));
        let start = i128::from(self.start_db100) + 10000;
        let end = i128::from(self.end_db100) + 10000;
        ((start * i128::from(total - count) + end * i128::from(count)) / i128::from(total) - 10000)
            as i32
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub struct AudioGainEnvelopeInfo {
    pub spec: AudioGainEnvelopeSpec,
    pub elapsed_steps: u32,
    pub phase_ms: u32,
    pub current_db100: i32,
    pub active: bool,
}
