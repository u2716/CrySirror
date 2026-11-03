//! Worker-ready channel selection for immutable engine hit coverage.
use super::*;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CoverageChannel {
    Alpha,
    Red,
    MaxRgb,
    Zero,
}
impl WidgetHitMask {
    /// Crop/pad a source at its local origin. No scaling or VM pixel policy is
    /// inferred. A smaller source leaves the remaining hit region transparent.
    pub fn from_rgba_channels(
        size: [u32; 2],
        source_size: [u32; 2],
        rgba: &[u8],
        channel: CoverageChannel,
        threshold: u8,
        check: &dyn Fn() -> Result<()>,
    ) -> Result<Self> {
        let count = u64::from(size[0]) * u64::from(size[1]);
        ensure!(
            size.iter().all(|v| *v > 0 && *v <= 8192) && count <= 16 * 1024 * 1024,
            "hit coverage dimensions"
        );
        let source_len = u64::from(source_size[0]) * u64::from(source_size[1]) * 4;
        ensure!(
            source_len == rgba.len() as u64,
            "hit coverage source length"
        );
        let mut coverage = Vec::new();
        coverage.try_reserve_exact(count as usize)?;
        coverage.resize(count as usize, 0);
        for y in 0..size[1].min(source_size[1]) {
            check()?;
            for x in 0..size[0].min(source_size[0]) {
                let at = ((y * source_size[0] + x) * 4) as usize;
                let pixel = &rgba[at..at + 4];
                coverage[(y * size[0] + x) as usize] = match channel {
                    CoverageChannel::Alpha => pixel[3],
                    CoverageChannel::Red => pixel[0],
                    CoverageChannel::MaxRgb => pixel[0].max(pixel[1]).max(pixel[2]),
                    CoverageChannel::Zero => 0,
                };
            }
        }
        Ok(Self {
            size,
            coverage: Arc::from(coverage),
            threshold,
            strict_bounds: false,
        })
    }
}
