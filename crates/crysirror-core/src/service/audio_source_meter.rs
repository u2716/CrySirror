// SPDX-License-Identifier: MPL-2.0

/// An opt-in stereo mean absolute PCM16 meter configured in source-independent
/// time. The mixer accumulates one observation per window on its output clock.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AudioSourceMeterSpec {
    /// Reference frames per second, from 8,000 to 192,000.
    pub reference_rate: u32,
    /// Frames in one window at the reference rate, up to one second.
    pub window_frames: u32,
    /// History capacity, from one to eight windows. Reads return the window at
    /// the next write index. Unfilled history entries read as zero.
    pub history_windows: u8,
}
