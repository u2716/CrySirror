// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! CG and effect137 address one native picture bank, never copied CG caches.
use super::*;
impl NativeAdapter {
    pub(crate) fn observe_png_header(
        &mut self,
        header: &crysirror_image::png_header::PngHeader,
    ) -> Result<()> {
        self.bridge.observe_png_header(header)
    }
    pub(crate) fn animation_bank(&self) -> Option<crate::native_animation_session::Bank> {
        self.bridge.animation_bank()
    }
    pub(crate) fn animation_context(&self) -> Result<crate::native_picture_reload::ContextData> {
        self.bridge.animation_context()
    }
    pub(crate) fn adopt_animation_bank(
        &mut self,
        bank: crate::native_animation_session::Bank,
    ) -> Result<()> {
        self.bridge.adopt_animation_bank(bank)
    }
}
