//! External owner-clock stereo balance in hundredths of a decibel. This is
//! independent of the equal-power pan/spatial and per-channel gain controls.
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum AudioBalanceCommand {
    /// Set the applied balance. A retained external track continues unless
    /// cancel_track is true; this supports independent immediate controls.
    Set { db100:i32, cancel_track:bool },
    /// Start from the applied value. finish_previous first applies the old
    /// track's endpoint. Steps are an external divisor, never milliseconds.
    Begin { track:u64, target_db100:i32, steps:u32, finish_previous:bool },
    /// Monotonic cumulative updates since Begin. PCM/device clocks never tick it.
    Advance { track:u64, updates:u64 },
}
#[derive(Clone,Copy,Debug,Default,PartialEq,Eq)]
pub struct AudioBalanceInfo {
    pub track:u64,
    pub db100:i32,
    pub target_db100:i32,
    pub completed_updates:u64,
    pub active:bool,
}
