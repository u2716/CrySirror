//! Native bitmap-font policy only; pixel operations use the shared Image API.
use super::*;
impl Bridge {
    pub(crate) fn user_gaiji_lease_count(&self) -> usize {
        self.user_gaiji_paths.len()
    }
    pub(crate) fn user_gaiji_sizes(&self) -> Result<BTreeSet<u16>> {
        let text = self
            .text
            .as_ref()
            .context("NNN user gaiji text capability absent")?;
        ensure!(
            text.profile.single_byte_map.is_none(),
            "NNN this user-gaiji extension profile uses two-byte native cells"
        );
        let mut sizes = BTreeSet::from([text.profile.body.size, text.profile.name.size]);
        if let Some(cutin) = &text.profile.cutin {
            sizes.extend(cutin.font_sizes(i32::from(text.profile.body.size))?);
        }
        if let Some(ruby) = &text.profile.ruby {
            for size in sizes.clone() {
                sizes.insert(u16::try_from(i32::from(size) * ruby.percent / 100)?);
            }
        }
        Ok(sizes)
    }
    pub(crate) fn install_user_gaiji(
        &mut self,
        mapping: BTreeMap<u16, u8>,
        images: BTreeMap<(u8, u16), String>,
        paths: Vec<String>,
    ) -> Result<()> {
        ensure!(
            !self.dependency_pending(),
            "NNN user font installation overlaps a native layout/frame job"
        );
        let text = self
            .text
            .as_mut()
            .context("NNN user gaiji text capability absent")?;
        text.profile.inline.user_gaiji = mapping;
        text.profile.inline.images.extend(images);
        // Published GlyphRuns/snapshots can still refer to the prior font.
        // Retain those immutable client-scoped leases through session teardown;
        // the producer enforces a bound before admitting a new generation.
        self.user_gaiji_paths.extend(paths);
        Ok(())
    }
}
