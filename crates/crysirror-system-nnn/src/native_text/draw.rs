// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Reviewed legacy Draw view:4ff1a0 draws the checked body window only. The
//! later drawWindowWithMessage/nextType glyph branch is absent from this ABI.
use super::*;
#[derive(Clone)]
pub(crate) struct DrawView {
    pub root: u32,
    pub hide: Vec<u32>,
    pub paints: Vec<crysirror_core::image_processing::ImageCompositeLayer>,
}
impl NativeTextBridge {
    pub(crate) fn draw_skip_click(&self, input: InputFrame) -> bool {
        use crysirror_core::service::buttons as B;
        input.buttons_pressed
            & (B::MOUSE_LEFT
                | B::ENTER
                | if self.wheel_down_click {
                    B::WHEEL_DOWN
                } else {
                    0
                })
            != 0
    }

    pub(crate) fn draw_view(&self, visible: bool) -> Result<DrawView> {
        ensure!(
            self.pending.is_none() && !self.dependency_pending() && self.ready.is_none(),
            "NNN DRAW requires drained message state"
        );
        let mut hide = Vec::new();
        if self.layers_present {
            hide.extend([self.profile.body_layer, self.profile.name_layer]);
            if self.profile.cutin.is_some() {hide.extend(cutin::node_ids(&self.profile)?);}
        }
        if self.windows_present {
            hide.extend(
                self.profile
                    .window
                    .iter()
                    .chain(self.profile.name_window.iter())
                    .map(|window| window.id),
            );
        }
        let paints = if visible {
            //534fa0 tests the global messageWindow flag and invokes the
            // configured body window with nameFlag0. No old glyph is printed.
            if let Some(window) = self.profile.window.as_ref() {
                background::window_layers(std::slice::from_ref(window))?
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };
        Ok(DrawView {
            root: self.profile.root,
            hide,
            paints,
        })
    }
    pub(crate) fn accept_draw_mode(&mut self, visible: bool) -> Result<()> {
        ensure!(
            self.pending.is_none() && !self.dependency_pending(),
            "NNN DRAW mode while message dependency active"
        );
        self.state.mode = 0x22220004;
        self.state.window_visible = visible;
        Ok(())
    }
}
