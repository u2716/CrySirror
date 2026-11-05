// SPDX-License-Identifier: MPL-2.0
//! Explicit offscreen presenter observations, without consulting a native window.
//! The virtual monitor is fixed at 1920×1080; its undecorated surface starts at
//! the project canvas size. UI requests are applied at the next host tick, so a
//! VM still waits for a presenter observation rather than acknowledging itself.
use crysirror_core::service::{DisplayInfo, WindowGeometry};
use crysirror_runtime::Runtime;

pub(crate) struct VirtualWindow {
    windowed_size: [u32; 2],
    windowed_position: [i32; 2],
    fullscreen: bool,
    visible: bool,
    minimized: bool,
}

impl VirtualWindow {
    const SCREEN: [u32; 2] = [1920, 1080];

    pub(crate) fn new(canvas: [u32; 2]) -> Self {
        Self {
            windowed_size: canvas,
            windowed_position: [0, 0],
            fullscreen: false,
            visible: true,
            minimized: false,
        }
    }

    /// Publish before bootstrap as well as before every admitted host tick.
    pub(crate) fn publish(&self, rt: &mut Runtime) {
        let (size, position) = if self.fullscreen {
            (Self::SCREEN, [0, 0])
        } else {
            (self.windowed_size, self.windowed_position)
        };
        rt.host.display = Some(DisplayInfo {
            screen: Self::SCREEN,
            work: [0, 0, Self::SCREEN[0] as i32, Self::SCREEN[1] as i32],
        });
        rt.host.window_geometry = Some(WindowGeometry {
            inner_size: size,
            outer_size: size,
            outer_position: Some(position),
        });
        rt.host.window_fullscreen = Some(self.fullscreen);
        rt.host.window_focused = Some(self.visible && !self.minimized);
        rt.host.window_minimized = Some(self.minimized);
        rt.host.engine.ui.pointer_surface = size;
    }

    /// The previous VM tick owns these requests; the presenter applies them
    /// before the next observation. Window style does not introduce fictitious
    /// OS borders into this explicitly undecorated virtual surface.
    pub(crate) fn begin_tick(&mut self, rt: &mut Runtime) {
        let ui = &mut rt.host.engine.ui;
        if let Some((_, fullscreen)) = ui.fullscreen {
            self.fullscreen = fullscreen;
        }
        if let Some((_, size)) = ui.window_requests.size.take() {
            self.windowed_size = size;
        }
        if let Some((_, position)) = ui.window_requests.position.take() {
            self.windowed_position = position;
        }
        if let Some((_, visible)) = ui.window_requests.visible.take() {
            self.visible = visible;
        }
        if let Some((_, minimized)) = ui.window_requests.minimized.take() {
            self.minimized = minimized;
        }
        self.publish(rt);
    }
}
