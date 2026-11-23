// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Native KIRA constructor/SetParam boundaries. Bitmap reads and shared image
//! preparation complete before this class is published to a native session.
use super::*;
pub(super) struct KiraSet {
    pub class: crate::native_kira::KiraClass,
    pub variables: crate::expression::Variables,
}
impl Bridge {
    pub(crate) fn configure_kira_picture(
        &mut self,
        picture: Option<crate::image_bridge::NativePictureViews>,
    ) -> Result<()> {
        ensure!(
            !self.dependency_pending(),
            "NNN KIRA constructor binding during native preparation"
        );
        if let Some(picture) = &picture {
            ensure!(
                [&picture.raw, &picture.opaque, &picture.keyed]
                    .iter()
                    .all(|image| image.client() == self.client),
                "NNN KIRA constructor foreign image"
            );
        }
        self.kira_class.picture(picture);
        Ok(())
    }
    pub(super) fn prepare_kira_set(
        &self,
        machine: &Machine,
        call: &NativeCall,
        p: &[i32; 6],
    ) -> Result<KiraSet> {
        let mut variables = machine.native_variables_candidate(call)?;
        let mut random = variables
            .random_state()
            .context("NNN KIRA requires original initialized random state")?;
        let mut class = self.kira_class.clone();
        class.set(p, self.bindings.size, &mut random)?;
        variables.replace_random_state(random)?;
        Ok(KiraSet { class, variables })
    }
}
