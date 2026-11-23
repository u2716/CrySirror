// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Config uses original native picture aliases, not an independent side bank.
//! ImageBridge/Reload retain native metadata; shared resources own the pixels.
use super::*;
use crate::native_config_layout::{
    allocation::Slot,
    pictures::{Key, Picture},
};
#[derive(Clone)]
pub(crate) struct ContextData {
    pub pictures: BTreeMap<Key, Picture>,
    pub reload: crate::native_picture_reload::ContextData,
    pub file: crate::native_system_file::NativeSystemFile,
    pub mask: i32,
    pub voices: Arc<crate::native_voice_policy::VoicePolicy>,
    /// PrintMessageWindowDirect inputs: the bound windows (body, name) and
    /// the PRINT body/name faces. Paths/resources stay owned by the text owner.
    pub windows: Vec<crate::native_text::MessageWindow>,
    pub faces: Option<(crate::native_text::TextFace, crate::native_text::TextFace)>,
    pub window_variants: i32,
}
impl Bridge {
    pub(crate) fn config_ui_context(&self) -> Result<ContextData> {
        ensure!(
            !self.dependency_pending() && self.actor_frame.is_none(),
            "NNN Config picture handoff requires a drained native actor"
        );
        let images = self
            .images
            .as_ref()
            .context("NNN Config image service absent")?;
        let bank = crate::native_staff_bank::snapshot(
            self.client,
            i32::from(self.bindings.layers),
            &images.pictures,
            images.retained_native_caches(),
        )?;
        let mut pictures = images.ui_pictures.clone();
        // Always refresh Effect aliases from the active script/native bank.
        // A previous Config session cannot shadow a subsequent SPT load.
        for (slot, cache) in bank {
            pictures.insert(
                Key::Allocated(Slot::Effect(u8::try_from(slot)?)),
                Picture::inherited(true, cache),
            );
        }
        let (windows, faces) = match self.text.as_ref() {
            Some(text) => (
                text.profile
                    .window
                    .iter()
                    .chain(text.profile.name_window.iter())
                    .cloned()
                    .collect(),
                Some((text.profile.body.clone(), text.profile.name.clone())),
            ),
            None => (vec![], None),
        };
        let window_variants = match self.window_binding.as_ref() {
            Some(binding) => binding.config_variants()?,
            None => 1,
        };
        Ok(ContextData {
            windows,
            faces,
            window_variants,
            pictures,
            mask: self.configuration_mask,
            reload: images.staff_context(),
            file: self
                .system_file
                .clone()
                .context("NNN Config SystemFile absent")?,
            voices: self
                .voice_policy
                .clone()
                .context("NNN Config voice policy absent")?,
        })
    }
    pub(crate) fn commit_config_pictures(
        &mut self,
        pictures: &BTreeMap<Key, Picture>,
    ) -> Result<()> {
        ensure!(
            !self.dependency_pending() && self.actor_frame.is_none(),
            "NNN Config picture publication while native work is in flight"
        );
        let mut bank = BTreeMap::new();
        let mut touched = BTreeSet::new();
        for (key, picture) in pictures {
            ensure!(
                picture.cache.client() == self.client,
                "NNN Config picture owner mismatch"
            );
            if let Key::Allocated(Slot::Effect(slot)) = key {
                bank.insert(i32::from(*slot), picture.cache.clone());
                touched.insert(i32::from(*slot));
            }
        }
        let images = self
            .images
            .as_mut()
            .context("NNN Config image service absent")?;
        images.adopt_native_caches(&bank, &touched, &self.not_trans);
        images.ui_pictures = pictures
            .iter()
            .filter(|(key, _)| !matches!(key, Key::Allocated(Slot::Effect(_))))
            .map(|(key, picture)| (key.clone(), picture.clone()))
            .collect();
        Ok(())
    }
}
