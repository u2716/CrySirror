// Copyright (c) 2026 CrySirror contributors.
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. A copy is available at https://mozilla.org/MPL/2.0/.
//! Original LayerManager hover/capture policy; hit testing remains the existing
//! native semantic query over engine-owned Layer/Canvas resources.
// LayerManager dispatch ordering is observable by script callbacks. Preserve
// Leave/re-hit/Enter and the bounded corrective hit after Enter changes a target.
// Hover, press capture and explicit release are separate state. Hiding a tree
// is not equivalent to detaching it; a retired target must receive no callbacks.
use crate::natives::Rt;
use crysirror_tjs2::{Result, runtime::ObjectHandle};

#[derive(Default)]
pub(crate) struct MouseState {
    pub hover: Option<usize>,
    pub capture: Option<usize>,
    pub release_called: bool,
    pub down_point: Option<[i64; 2]>,
    pub last_recheck: u64,
}

pub(crate) fn release(rt: &mut Rt) {
    rt.host_mut().mouse.capture = None;
    rt.host_mut().mouse.release_called = true;
}

/// BlurTree clears LastMouseMoveSent before firing Leave. Part additionally
/// releases capture after that callback; hiding/disable alone does not.
pub(crate) fn leave_tree(rt: &mut Rt, root: usize, part: bool) -> Result<()> {
    let hover = rt.host().mouse.hover.filter(|h| rt.host().is_ancestor_or_self(*h, root));
    if let Some(h) = hover {
        rt.host_mut().mouse.hover = None;
        let resumable = rt.host().can_suspend;
        rt.host_mut().can_suspend = false;
        let result = rt.call_object_method(ObjectHandle(h), "onMouseLeave", Vec::new());
        rt.host_mut().can_suspend = resumable;
        result?;
    }
    if part && rt.host().mouse.capture.is_some_and(|h| rt.host().is_ancestor_or_self(h, root)) {
        release(rt);
    }
    Ok(())
}
