use crate::native_vm::sprite::{PalAnimationAxis, PalAnimationFlags, PalRect, SpriteHandle, SpriteSystem};
use crate::native_vm::task::TaskUpdateOutcome;

/// Re-export: animation handles are task handles.
pub use crate::native_vm::task::AnimationHandle;

// ---- Descriptor types (used to create animation tasks via TaskSystem) ----

#[derive(Clone, Debug)]
pub struct PalSheetAnimationDesc {
    pub sprite: SpriteHandle,
    pub flags: PalAnimationFlags,
    pub frame_delay_ms: u32,
    pub running: bool,
}

impl PalSheetAnimationDesc {
    pub fn horizontal(sprite: SpriteHandle, frame_delay_ms: u32) -> Self {
        Self {
            sprite,
            flags: PalAnimationAxis::Horizontal.into(),
            frame_delay_ms,
            running: true,
        }
    }

    pub fn vertical(sprite: SpriteHandle, frame_delay_ms: u32) -> Self {
        Self {
            sprite,
            flags: PalAnimationAxis::Vertical.into(),
            frame_delay_ms,
            running: true,
        }
    }
}

#[derive(Clone, Debug)]
pub struct PalSequenceAnimationDesc {
    pub sprite: SpriteHandle,
    pub frames: Vec<PalAnimationFrameRecord>,
    pub running: bool,
}

/// 24-byte frame record for PalAnimationEx. Mirrors the original structure:
/// RECT.left, RECT.top, RECT.right, RECT.bottom, duration_ms, next_frame_index.
#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PalAnimationFrameRecord {
    pub rect: PalRect,
    pub duration_ms: u32,
    /// -1 = terminal (last frame).
    pub next: i32,
}

const _: () = {
    use core::mem::{align_of, offset_of, size_of};
    assert!(size_of::<PalAnimationFrameRecord>() == 24);
    assert!(align_of::<PalAnimationFrameRecord>() == 4);
    assert!(offset_of!(PalAnimationFrameRecord, duration_ms) == 16);
    assert!(offset_of!(PalAnimationFrameRecord, next) == 20);
};

impl PalAnimationFrameRecord {
    pub const fn new(rect: PalRect, duration_ms: u32, next: i32) -> Self {
        Self {
            rect,
            duration_ms,
            next,
        }
    }
}

// ---- PalSheetAnimation: 0x1C task data ----

/// Runtime state for a sprite-sheet animation. Corresponds to PalAnimation task data (0x1C bytes).
/// Stored inside TaskSystem as TaskKind::AnimationSheet.
#[derive(Clone, Debug)]
pub struct PalSheetAnimation {
    /// Native constructors install an initializer before the update callback.
    initialized: bool,
    pub sprite: SpriteHandle,
    pub frame_delay_ms: u32,
    /// PAL-time timestamp of the last frame advance (PaltimeGetTime equivalent).
    pub last_time_ms: u32,
    pub current_frame: u16,
    pub frame_count: u16,
    pub flags: PalAnimationFlags,
    pub running: bool,
    pub looped_since_last_query: bool,
}

impl PalSheetAnimation {
    /// Create a new sheet animation. Does NOT reset sprite source_rect.
    /// `pal_time_ms` is the current PAL cached time.
    pub fn create(
        sprites: &mut SpriteSystem,
        desc: PalSheetAnimationDesc,
        _pal_time_ms: u32,
    ) -> anyhow::Result<Self> {
        let flags = PalAnimationFlags::from_original(desc.flags.raw());
        sprites.get(desc.sprite).ok_or_else(|| {
            anyhow::anyhow!("PalAnimation sprite {:?} does not exist", desc.sprite)
        })?;
        Ok(Self {
            initialized: false,
            sprite: desc.sprite,
            frame_delay_ms: desc.frame_delay_ms,
            last_time_ms: 0,
            current_frame: 0,
            frame_count: 0,
            flags,
            running: true,
            looped_since_last_query: false,
        })
    }

    /// Advance at most one frame. Called once per TaskSystem.process() call.
    ///
    /// Original update rule (sub_102394E0):
    ///   elapsed = PaltimeGetTime() - last_time
    ///   if elapsed < frame_delay_ms: return
    ///   current_frame += 1; if == frame_count: current_frame = 0; looped = 1
    ///   PalSpriteRectSetPos(sprite, ...)
    ///   last_time = PaltimeGetTime()   (not accumulated remainder)
    pub(crate) fn step_once(
        &mut self,
        sprites: &mut SpriteSystem,
        pal_time_ms: u32,
    ) -> TaskUpdateOutcome {
        if !self.initialized {
            // PalAnimation's first callback samples dimensions and time even
            // when the caller stopped the animation before it was scheduled.
            self.frame_count = sprites.get(self.sprite).map_or(0, |sprite| {
                let (dimension, cell) = match self.flags.axis() {
                    PalAnimationAxis::Horizontal => {
                        (sprite.texture_size.width, sprite.cell_size.width)
                    }
                    PalAnimationAxis::Vertical => {
                        (sprite.texture_size.height, sprite.cell_size.height)
                    }
                };
                native_frame_count(dimension, cell)
            });
            self.current_frame = 0;
            self.looped_since_last_query = false;
            self.last_time_ms = pal_time_ms;
            self.initialized = true;
            return TaskUpdateOutcome::Continue;
        }
        if !self.running {
            return TaskUpdateOutcome::Continue;
        }
        let elapsed = pal_time_ms.wrapping_sub(self.last_time_ms);
        if self.frame_delay_ms != 0 && elapsed < self.frame_delay_ms {
            return TaskUpdateOutcome::Continue;
        }
        self.current_frame = self.current_frame.wrapping_add(1);
        if self.current_frame == self.frame_count {
            self.current_frame = 0;
            self.looped_since_last_query = true;
        }
        if let Err(e) = self.apply_rect(sprites) {
            log::warn!("PalAnimation step_once: {e}");
        }
        self.last_time_ms = pal_time_ms;
        TaskUpdateOutcome::Continue
    }

    /// Reset to frame 0 without changing `running`. Matches PalAnimationReset_0.
    pub fn reset(&mut self, sprites: &mut SpriteSystem, pal_time_ms: u32) -> anyhow::Result<()> {
        self.current_frame = 0;
        self.last_time_ms = pal_time_ms;
        self.looped_since_last_query = false;
        sprites
            .rect_set_pos(self.sprite, 0, 0)
            .then_some(())
            .ok_or_else(|| {
                anyhow::anyhow!("PalAnimationReset sprite {:?} does not exist", self.sprite)
            })
    }

    fn apply_rect(&self, sprites: &mut SpriteSystem) -> anyhow::Result<()> {
        match self.flags.axis() {
            PalAnimationAxis::Horizontal => {
                sprites.rect_set_pos(self.sprite, i32::from(self.current_frame), 0)
            }
            PalAnimationAxis::Vertical => {
                sprites.rect_set_pos(self.sprite, 0, i32::from(self.current_frame))
            }
        }
        .then_some(())
        .ok_or_else(|| anyhow::anyhow!("PalAnimation sprite {:?} does not exist", self.sprite))
    }
}

/// The native initializer performs signed division and stores the low word.
/// A zero divisor or signed division overflow faults natively; keep those
/// malformed dimensions inert in the portable engine instead.
fn native_frame_count(dimension: u32, cell: u32) -> u16 {
    (dimension as i32).checked_div(cell as i32).unwrap_or(0) as u16
}

// ---- PalSequenceAnimation: 0x18 task data ----

/// Runtime state for a frame-sequence animation. Corresponds to PalAnimationEx task data (0x18 bytes).
/// Stored inside TaskSystem as TaskKind::AnimationSequence.
#[derive(Clone, Debug)]
pub struct PalSequenceAnimation {
    initialized: bool,
    pub sprite: SpriteHandle,
    pub frames: Vec<PalAnimationFrameRecord>,
    pub current_duration_ms: u32,
    /// PAL-time timestamp of the last frame advance.
    pub last_time_ms: u32,
    /// -1 = terminal state.
    pub current_frame: i32,
    pub running: bool,
}

impl PalSequenceAnimation {
    /// Create and apply frame 0. `pal_time_ms` is the current PAL cached time.
    pub fn create(
        sprites: &mut SpriteSystem,
        desc: PalSequenceAnimationDesc,
        _pal_time_ms: u32,
    ) -> anyhow::Result<Self> {
        let anim = Self {
            initialized: false,
            sprite: desc.sprite,
            frames: desc.frames,
            current_duration_ms: 0,
            last_time_ms: 0,
            current_frame: 0,
            running: true,
        };
        let frame = anim.frames.first().ok_or_else(|| {
            anyhow::anyhow!("PalAnimationEx requires at least one 24-byte frame record")
        })?;
        if !sprites.set_rect(anim.sprite, Some(frame.rect)) {
            return Err(anyhow::anyhow!(
                "PalAnimationEx sprite {:?} does not exist",
                anim.sprite
            ));
        }
        Ok(anim)
    }

    /// Advance at most one frame. Called once per TaskSystem.process() call.
    ///
    /// Original update rule (sub_10239620):
    ///   if !running || current_frame == -1: return
    ///   elapsed = PaltimeGetTime() - last_time
    ///   if elapsed < current_duration: return
    ///   current_frame = records[current_frame].next
    ///   current_duration = records[current_frame].duration  (read before -1 check in original)
    ///   if current_frame != -1: PalSpriteSetRect(...); last_time = now
    ///
    /// Rust: treat -1 as terminal and keep last frame rect without OOB read.
    pub(crate) fn step_once(
        &mut self,
        sprites: &mut SpriteSystem,
        pal_time_ms: u32,
    ) -> TaskUpdateOutcome {
        if !self.initialized {
            // Creation applies the first rectangle, but only this first
            // callback reads the duration and starts the elapsed-time clock.
            self.current_duration_ms = self
                .frames
                .get(self.current_frame as usize)
                .map_or(0, |frame| frame.duration_ms);
            self.last_time_ms = pal_time_ms;
            self.initialized = true;
            return TaskUpdateOutcome::Continue;
        }
        if !self.running || self.current_frame == -1 {
            return TaskUpdateOutcome::Continue;
        }
        let elapsed = pal_time_ms.wrapping_sub(self.last_time_ms);
        if elapsed < self.current_duration_ms {
            return TaskUpdateOutcome::Continue;
        }
        let next = match self.frames.get(self.current_frame as usize) {
            Some(f) => f.next,
            None => {
                log::warn!(
                    "PalAnimationEx: current_frame {} out of range for {} frames",
                    self.current_frame,
                    self.frames.len()
                );
                return TaskUpdateOutcome::Continue;
            }
        };
        self.current_frame = next;
        if self.current_frame == -1 {
            // Terminal: keep last rect, don't update last_time_ms.
            return TaskUpdateOutcome::Continue;
        }
        if let Err(e) = self.apply_current_frame(sprites, pal_time_ms) {
            log::warn!("PalAnimationEx step_once: {e}");
        }
        TaskUpdateOutcome::Continue
    }

    /// Reset to frame 0 and force running = true. Matches PalAnimationResetEx_0.
    pub fn reset(&mut self, sprites: &mut SpriteSystem, pal_time_ms: u32) -> anyhow::Result<()> {
        self.current_frame = 0;
        self.running = true;
        self.apply_current_frame(sprites, pal_time_ms)
    }

    /// Reset with a new frame table, restart from frame 0. Matches PalAnimationResetEx_0 variant.
    pub fn reset_with_frames(
        &mut self,
        sprites: &mut SpriteSystem,
        frames: Vec<PalAnimationFrameRecord>,
        pal_time_ms: u32,
    ) -> anyhow::Result<()> {
        self.frames = frames;
        self.reset(sprites, pal_time_ms)
    }

    fn apply_current_frame(
        &mut self,
        sprites: &mut SpriteSystem,
        pal_time_ms: u32,
    ) -> anyhow::Result<()> {
        if self.frames.is_empty() {
            return Err(anyhow::anyhow!(
                "PalAnimationEx requires at least one 24-byte frame record"
            ));
        }
        let frame = *self
            .frames
            .get(self.current_frame as usize)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "PalAnimationEx frame index {} out of range",
                    self.current_frame
                )
            })?;
        self.current_duration_ms = frame.duration_ms;
        self.last_time_ms = pal_time_ms;
        sprites
            .set_rect(self.sprite, Some(frame.rect))
            .then_some(())
            .ok_or_else(|| {
                anyhow::anyhow!("PalAnimationEx sprite {:?} does not exist", self.sprite)
            })
    }
}

// ---- Compatibility aliases ----
pub type SheetAnimationDesc = PalSheetAnimationDesc;
pub type SequenceAnimationDesc = PalSequenceAnimationDesc;
pub type AnimationFrameRecord = PalAnimationFrameRecord;
pub type SheetAnimationAxis = PalAnimationAxis;
