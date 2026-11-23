// SPDX-License-Identifier: MPL-2.0
//! Playback of the VM's step list (0x14002a680). Each frame runs records until a wait
//! is still pending; `0x2284` runs the rest at once, without crossfades or tweens, and
//! clears the list. Image records load on a worker; the original loads them inline,
//! so their replacements land when the worker finishes, in record order.

use crate::vm::{CmvsVm, SequenceKind};

/// Work a run hands to the adapter.
pub(super) enum Action {
    /// Script object image replaced with a crossfade (0x140029fe0).
    Image { slot: usize, name: String, fade: u32 },
    /// Part of message window 0, or its root for a negative part (0x140029e70).
    Window { part: i32, name: String, fade: u32 },
    /// World position track to `target`; `instant` applies it at once (sub 1 only).
    World { slot: usize, mode: u32, duration: u32, target: [f32; 3], instant: bool },
    Camera { mode: u32, duration: u32, target: [f32; 3] },
}

/// Runs records from the current index. `instant` is the flush pass of `0x2284`.
pub(super) fn run(vm: &mut CmvsVm, instant: bool, now: u32, exists: impl Fn(usize) -> bool) -> Vec<Action> {
    let mut actions = Vec::new();
    let sequence = vm.sequence_mut();
    if !sequence.running || sequence.index >= sequence.count() {
        return actions;
    }
    while let Some(step) = sequence.step(sequence.index).cloned() {
        match step.kind {
            SequenceKind::Wait if !instant => {
                let elapsed = now.wrapping_sub(sequence.waited).wrapping_sub(sequence.start);
                if elapsed < step.duration {
                    return actions;
                }
                sequence.waited = sequence.waited.wrapping_add(step.duration);
            }
            SequenceKind::Object if exists(step.slot as usize) => {
                let slot = step.slot as usize;
                match step.sub {
                    0 => {
                        if let Some(name) = step.name() {
                            let fade = if instant { 0 } else { step.duration };
                            actions.push(Action::Image { slot, name, fade });
                        }
                    }
                    1 | 2 if !instant || step.sub == 1 => actions.push(Action::World {
                        slot,
                        mode: step.mode,
                        duration: step.duration,
                        target: step.position(),
                        instant,
                    }),
                    _ => {}
                }
            }
            SequenceKind::Camera => {
                let duration = if instant { 0 } else { step.duration };
                actions.push(Action::Camera { mode: step.sub, duration, target: step.position() });
            }
            SequenceKind::Window if step.sub == 0 => {
                if let Some(name) = step.name() {
                    let fade = if instant { 0 } else { step.duration };
                    actions.push(Action::Window { part: step.slot as i32, name, fade });
                }
            }
            _ => {}
        }
        sequence.index += 1;
        if sequence.index >= sequence.count() {
            break;
        }
    }
    sequence.running = false;
    actions
}
