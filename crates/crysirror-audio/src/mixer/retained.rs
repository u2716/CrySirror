// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Retained canonical voice transitions, never a second mixer/source instance.
use super::*;
/// True means the same slot/source must stay allocated. Explicit Stop is not
/// routed here: it still retires all source/effect ownership.
pub(super) fn source_finished(voice: &mut Voice) -> bool {
    if !voice.spec.retain_on_finish {
        return false;
    }
    if !voice.source_ended {
        finish_source(voice);
    }
    voice.spec.transport_active = false;
    true
}
pub(super) fn gain_finished(voice: &mut Voice) -> bool {
    if !voice.spec.retain_on_finish {
        return false;
    }
    voice.spec.transport_active = false;
    voice.gain_stop_after_fade = false;
    true
}
/// Parked transport produces no source/DSP samples, but independent controls
/// advance on the output clock. Explicit Pause/dependency policies are applied
/// before choosing this path. Return true only when an ordinary voice retires.
pub(super) fn tick_inactive(voice: &mut Voice) -> bool {
    silent_transport_tick(voice);
    voice.gain.step();
    voice.pan.step();
    for gain in &mut voice.stereo_gain {
        gain.step();
    }
    voice.gain_stop_after_fade && voice.gain.left == 0 && !gain_finished(voice)
}
