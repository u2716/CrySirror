// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! WATERDROP95 is a full-screen RGB fill in the registered original source,
//! despite its name. It is not a displacement or particle effect.
use crysirror_core::image_processing::ImageInput;
pub(crate) fn parameters(values: &[i32]) -> [i32; 3] {
    let r = values.first().copied().unwrap_or(0);
    let g = values.get(1).copied().unwrap_or(r);
    [r, g, values.get(2).copied().unwrap_or(g)]
}
pub(crate) fn image_input(p: [i32; 3], canvas: [u32; 2]) -> ImageInput {
    ImageInput::Solid {
        size: canvas,
        color: [p[0] as u8, p[1] as u8, p[2] as u8, 255],
    }
}
// Source SetParam stores three RGB words, count0/countMax9999. CountIsMax
// returns FALSE; Init/BeginPrint/Calcu/End are empty. CheckNeedPicPrint and
// CheckAllPrint both return FALSE: do not add an ordinary picture underlay or
// misinterpret parameter0 as the counter duration during raw save hydration.
