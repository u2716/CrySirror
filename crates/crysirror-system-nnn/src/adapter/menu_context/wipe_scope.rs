// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Native RNG/table ownership for graph-based mode painters. No worker may
//! own a concurrent Variables candidate when a mode takes this snapshot.
use super::*;
impl NativeAdapter {
    pub(crate) fn retained_cg_difference_picture(
        &self,
    ) -> Option<Arc<dyn crysirror_core::image_processing::ImageResource>> {
        self.bridge.retained_cg_difference_picture()
    }
    pub(crate) fn retain_cg_difference_picture(
        &mut self,
        image: Arc<dyn crysirror_core::image_processing::ImageResource>,
    ) -> Result<()> {
        self.bridge.retain_cg_difference_picture(image)
    }
    pub(crate) fn acquire_mode_wipe_context(
        &self,
    ) -> Result<
        Option<(
            crysirror_image::temporal_pattern::PatternRng,
            crate::native_simple_wipe::Tables,
        )>,
    > {
        if self.worker.is_some() {
            return Ok(None);
        }
        let variables = self.modal_variables()?;
        let random = variables
            .random_state()
            .context("NNN native mode wipe RNG absent")?;
        Ok(Some((random, self.bridge.simple_wipe_tables())))
    }
    /// Called after the exact shared image receipt is ready and published.
    /// A completion pump never reruns a wipe planner or its random draws.
    pub(crate) fn accept_mode_wipe_context(
        &mut self,
        random: crysirror_image::temporal_pattern::PatternRng,
        tables: crate::native_simple_wipe::Tables,
    ) -> Result<()> {
        ensure!(
            self.worker.is_none(),
            "NNN mode wipe completion while VM worker owns variables"
        );
        let mut variables = self.modal_variables()?;
        variables.replace_random_state(random)?;
        self.publish_general_variables(variables)?;
        self.bridge.set_simple_wipe_tables(tables);
        Ok(())
    }
}
