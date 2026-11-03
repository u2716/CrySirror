// Copyright (c) xmoezzz. MPL-2.0.
//! Deterministic strategy simulation: tile navigation, tactical evaluation,
//! body relationships/sweeps and cubic paths. No VM or rendering dependency.
pub mod grid;
mod msvc_qsort;
pub mod slg;
pub mod space;
pub mod spline;

pub mod status {
    pub const NO_HANDLE: u32 = 0x8000_0000;
    pub const BAD_ARGUMENT: u32 = 0x8000_0001;
    pub const NO_MAP: u32 = 0x8000_0002;
    pub const NO_UNIT: u32 = 0x8000_0003;
    pub const OUT_OF_MAP: u32 = 0x8000_0004;
    pub const NO_TABLE: u32 = 0x8000_0005;
    pub const NO_SEARCH: u32 = 0x8000_0006;
    pub const UNREACHABLE: u32 = 0x8000_0007;
    pub const BAD_ORIGIN: u32 = 0x8000_0008;
    pub const BAD_HEIGHT: u32 = 0x8000_0009;
    pub const BAD_RADIUS: u32 = 0x8000_000A;
    pub const BAD_DIRECTION: u32 = 0x8000_000B;
    pub const FAILED: u32 = 0xFFFF_FFFF;
    pub const BROKEN_PATH: u32 = 0xFFFF_FFFE;
    pub const SEARCH_FAILED: u32 = FAILED;
    // DCTELogicalField's own codes, remapped by the manager.
    pub const NO_CELLS: u32 = 0x8000_0003;
    pub const NOT_REACHED: u32 = 0x8000_0004;
    pub const OUT_OF_FIELD: u32 = 0x8000_0005;
    pub const PRUNED: u32 = 0x8000_0006;
    pub const NO_THREAT_KEY: u32 = 0x8000_000D;
    pub const NO_THREAT_MAP: u32 = 0x8000_000E;
    // DCTESLGEvaluator.
    pub const BAD_COUNT: u32 = 0x9000_0002;
    pub const BAD_INDEX: u32 = 0x9000_0003;
}

pub fn float_to_fixed(value: f32) -> i32 {
    let scaled = (value * 65_536.0).round_ties_even();
    let raw = if scaled >= -2_147_483_648.0 && scaled < 2_147_483_648.0 {
        scaled as i32
    } else {
        i32::MIN
    };
    let whole = raw & !0xffff;
    let fraction = raw.wrapping_sub(whole);
    if fraction > 0xffc0 {
        whole.wrapping_add(0x10000)
    } else if fraction < 0x40 {
        whole
    } else {
        raw
    }
}
