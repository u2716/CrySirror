// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Explicit Source32 animation class admission and the shared CG/effect cache.
//! Legacy's absent class is not inferred from a setup key or another EXE ABI.
use super::*;
impl Bridge {
    /// Construct the public-source ANIMATION class on shared cancellable jobs.
    /// Source32 embedders call this before admitting native137 or CG animation;
    /// it does not change the reviewed MUGEN2 VM, save or constructor profile.
    pub fn poll_source32_animation_configuration(
        &mut self,
        setup: &crate::config::Names,
        host: &mut dyn HostServices,
    ) -> Result<bool> {
        let config = crate::native_animation_session::configuration(setup)?;
        if let Some(bank) = &self.animation {
            ensure!(
                bank.config() == &config,
                "NNN live ANIMATION configuration changed"
            );
            return Ok(true);
        }
        if self.animation_creation.is_none() {
            ensure!(
                !self.dependency_pending(),
                "NNN ANIMATION configuration during native dependency"
            );
            let context = self
                .images
                .as_ref()
                .context("NNN ANIMATION resource context absent")?
                .staff_context();
            let palette = self
                .images
                .as_ref()
                .context("NNN PNG loader history absent")?
                .png_palette
                .clone();
            self.animation_creation = Some(crate::native_animation_session::Creation::begin(
                self.client,
                &context,
                config.clone(),
                self.bindings.size,
                palette,
                host,
            )?);
        }
        let work = self.animation_creation.as_mut().unwrap();
        ensure!(
            work.config() == &config,
            "NNN pending ANIMATION configuration changed"
        );
        let Some(bank) = work.poll(host)? else {
            return Ok(false);
        };
        self.animation = Some(bank);
        self.animation_creation = None;
        Ok(true)
    }
    /// The all-effect's native animation-off flag changes frame selection, not
    /// the allocated cache bank or its retained filename/loadOk state.
    pub fn set_source_animation_off(&mut self, value: i32) {
        self.animation_off = value;
    }
    pub fn set_source_animation_tag(&mut self, name: &[u8]) -> Result<()> {
        self.animation
            .as_mut()
            .context("NNN ANIMATION class absent")?
            .set_tag(name);
        Ok(())
    }
    pub(crate) fn animation_bank(&self) -> Option<crate::native_animation_session::Bank> {
        let mut bank = self.animation.clone()?;
        if let Some(images) = self.images.as_ref() {
            bank.set_palette(images.png_palette.clone());
        }
        Some(bank)
    }
    pub(crate) fn adopt_animation_bank(
        &mut self,
        bank: crate::native_animation_session::Bank,
    ) -> Result<()> {
        let existing = self
            .animation
            .as_ref()
            .context("NNN ANIMATION owner absent")?;
        ensure!(
            existing.config() == bank.config(),
            "NNN ANIMATION bank configuration changed"
        );
        if let Some(palette) = bank.palette() {
            self.images
                .as_mut()
                .context("NNN PNG loader history absent")?
                .png_palette = palette.clone();
        }
        self.animation = Some(bank);
        Ok(())
    }
    pub(crate) fn observe_png_header(
        &mut self,
        header: &crysirror_image::png_header::PngHeader,
    ) -> Result<()> {
        self.images
            .as_mut()
            .context("NNN PNG loader history absent")?
            .png_palette
            .observe_header(header)
    }
    pub(crate) fn animation_context(&self) -> Result<crate::native_picture_reload::ContextData> {
        Ok(self
            .images
            .as_ref()
            .context("NNN ANIMATION resource context absent")?
            .staff_context())
    }
}
