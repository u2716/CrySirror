//! Reviewed native PAL ABI contracts and shared value layouts.
//!
//! The original DLL interface is Win32 x86 cdecl. Native function pointer
//! declarations are exposed only on that target. Portable sprite handles are
//! arena keys, never addresses that may be passed to an original DLL. The
//! separate `ffi` module defines Sena's platform-launcher API.
//!
//! Update reverse/abi/pal32.json before implementing a newly confirmed export,
//! then run reverse/abi/generate.py. Uncertain signatures stay unconfirmed.

#[rustfmt::skip]
mod generated;
pub use generated::*;
pub use crate::native_vm::save_format::PalSpriteInfoKoikake32;

/// Native WAVEFORMATEX copied by newer PalGetSoundBufferFormat.
#[repr(C, packed(2))]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PalWaveFormatEx32 {
    pub format_tag: u16,
    pub channels: u16,
    pub samples_per_second: u32,
    pub average_bytes_per_second: u32,
    pub block_align: u16,
    pub bits_per_sample: u16,
    pub extra_size: u16,
}

const _: () = {
    assert!(core::mem::size_of::<PalWaveFormatEx32>() == 18);
    assert!(core::mem::align_of::<PalWaveFormatEx32>() == 2);
    assert!(core::mem::offset_of!(PalWaveFormatEx32, samples_per_second) == 4);
    assert!(core::mem::offset_of!(PalWaveFormatEx32, block_align) == 12);
    assert!(core::mem::offset_of!(PalWaveFormatEx32, extra_size) == 16);
};

/// Native RECT: four signed DWORDs, in left/top/right/bottom order.
#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PalRect32 {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

/// Type-0 source copied by fixture 0x100B3E30.
/// Other FX types must define their own recovered layouts.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PalRasterSource32 {
    pub period_ms: u32,
    pub undulate_x: f32,
    pub undulate_y: f32,
}

/// Optional group-3 streaming range, in decoded PCM bytes.
/// A zero loop end selects the full decoded byte size.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PalSoundLoadRange32 {
    pub enabled: u32,
    pub initial_start_bytes: u32,
    pub loop_start_bytes: u32,
    pub loop_end_bytes: u32,
}

/// Native SIZE output used by PAL font measurement.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PalTextSize32 {
    pub cx: i32,
    pub cy: i32,
}

/// Native transition allocation. Pointer fields are original x86 addresses,
/// never portable arena handles or host-sized pointers.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PalSpriteTransition32 {
    pub transition_type: i32,
    pub state: i32,
    pub render_address: u32,
    pub first_sprite_address: u32,
    pub second_sprite_address: u32,
    pub last_frame: i32,
    pub duration_ms: u32,
    pub start_time_ms: u32,
}

/// Pointee identity only. The native sprite object's full layout is not
/// represented by the portable Rust PalSprite or SpriteHandle types.
pub enum PalSpriteOpaque {}

/// Native render-tree identity; portable Scene values are not ABI-compatible.
pub enum PalRenderOpaque {}

/// Native scheduler task identity, distinct from a portable task arena key.
pub enum PalTaskOpaque {}

/// Native button group identity; sprites remain borrowed by the group.
pub enum PalButtonGroupOpaque {}

/// Three original x86 sprite addresses returned by button reaction polling.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PalButtonReaction32 {
    pub completed_address: u32,
    pub hovered_address: u32,
    pub pending_address: u32,
}

/// The portable sequence uses the same reviewed, pointer-free record layout.
pub type PalAnimationFrame32 = crate::native_vm::animation::PalAnimationFrameRecord;

#[derive(Clone, Copy, Debug)]
pub struct NativeBuildEvidence {
    pub build: &'static str,
    pub address: u32,
    pub binary_sha256: &'static str,
    pub evidence: &'static str,
}

#[derive(Clone, Copy, Debug)]
pub struct AbiContract {
    pub name: &'static str,
    pub prototype: &'static str,
    pub stack_bytes: u16,
    pub implementation: &'static str,
    pub notes: &'static str,
    pub evidence: &'static [NativeBuildEvidence],
}

// These are native memory layouts, not tests mirroring portable algorithms.
const _: () = {
    use core::mem::{align_of, offset_of, size_of};
    assert!(size_of::<PalButtonReaction32>() == 12);
    assert!(align_of::<PalButtonReaction32>() == 4);
    assert!(offset_of!(PalButtonReaction32, pending_address) == 8);
    assert!(size_of::<PalRect32>() == 16);
    assert!(align_of::<PalRect32>() == 4);
    assert!(offset_of!(PalRect32, top) == 4);
    assert!(offset_of!(PalRect32, right) == 8);
    assert!(offset_of!(PalRect32, bottom) == 12);
    assert!(size_of::<PalRasterSource32>() == 12);
    assert!(align_of::<PalRasterSource32>() == 4);
    assert!(offset_of!(PalRasterSource32, undulate_x) == 4);
    assert!(offset_of!(PalRasterSource32, undulate_y) == 8);
    assert!(size_of::<PalSoundLoadRange32>() == 16);
    assert!(align_of::<PalSoundLoadRange32>() == 4);
    assert!(offset_of!(PalSoundLoadRange32, initial_start_bytes) == 4);
    assert!(offset_of!(PalSoundLoadRange32, loop_start_bytes) == 8);
    assert!(offset_of!(PalSoundLoadRange32, loop_end_bytes) == 12);
    assert!(size_of::<PalTextSize32>() == 8);
    assert!(align_of::<PalTextSize32>() == 4);
    assert!(offset_of!(PalTextSize32, cy) == 4);
    assert!(size_of::<PalSpriteTransition32>() == 32);
    assert!(align_of::<PalSpriteTransition32>() == 4);
    assert!(offset_of!(PalSpriteTransition32, state) == 4);
    assert!(offset_of!(PalSpriteTransition32, render_address) == 8);
    assert!(offset_of!(PalSpriteTransition32, first_sprite_address) == 12);
    assert!(offset_of!(PalSpriteTransition32, second_sprite_address) == 16);
    assert!(offset_of!(PalSpriteTransition32, last_frame) == 20);
    assert!(offset_of!(PalSpriteTransition32, duration_ms) == 24);
    assert!(offset_of!(PalSpriteTransition32, start_time_ms) == 28);
};
