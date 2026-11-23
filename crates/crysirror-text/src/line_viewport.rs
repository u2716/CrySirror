//! Integer-row viewport selection, independent of any engine's history policy.
use anyhow::{Result, ensure};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LineViewport {
    pub first: u32,
    pub end: u32,
    pub offset: u32,
}
impl LineViewport {
    /// `position` is the fraction from the first to the last visible window.
    /// Half-row ties use nearest-even, matching rounded native scrollbar rows.
    pub fn select(total: u32, visible: u32, pitch: u32, position: f32) -> Result<Self> {
        ensure!(
            visible > 0 && pitch > 0 && position.is_finite(),
            "line viewport bounds"
        );
        let first = (f64::from(total.saturating_sub(visible)) * f64::from(position.clamp(0., 1.)))
            .round_ties_even() as u32;
        let offset = first
            .checked_mul(pitch)
            .ok_or_else(|| anyhow::anyhow!("line viewport offset overflow"))?;
        Ok(Self {
            first,
            end: first.saturating_add(visible).min(total),
            offset,
        })
    }
}
