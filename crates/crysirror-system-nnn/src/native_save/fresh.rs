// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Fresh legacy envelope production. Every native Save* begins with memset;
//! unused padding is initialized instead of depending on previous heap bytes.
use super::*;
use crate::{native_system_file::SystemStamp, native_thumbnail::NativeThumbnail};
impl LegacySaveProfile {
    pub(crate) fn validate_live_actors(&self) -> Result<()> {
        ensure!(
            !self.omake && !self.cutin && self.extension_sizes.is_empty(),
            "NNN live save needs native Omake/Cutin/extension actor producers"
        );
        Ok(())
    }
}
impl NativeSaveFile {
    pub(crate) fn for_live(
        profile: &LegacySaveProfile,
        stamp: SystemStamp,
        thumbnail: NativeThumbnail,
    ) -> Result<Self> {
        // These actors own their bytes. Never pass uninitialized/zero extension
        // data off as a working live producer for an active Omake/Cutin class.
        profile.validate_live_actors()?;
        ensure!(
            thumbnail.size.iter().all(|n| *n > 0)
                && u64::from(thumbnail.size[0]) * u64::from(thumbnail.size[1]) * 4
                    == thumbnail.bgrx.len() as u64,
            "NNN live MINICG layout"
        );
        let mut info = vec![0; 32];
        info[0..4].copy_from_slice(&8u32.to_le_bytes());
        info[4..8].copy_from_slice(&profile.version.to_le_bytes());
        let mut mini = Vec::with_capacity(16 + thumbnail.bgrx.len());
        mini.extend(thumbnail.size[0].to_le_bytes());
        mini.extend(thumbnail.size[1].to_le_bytes());
        mini.extend([0; 8]);
        mini.extend(thumbnail.bgrx);
        let mut file = Self {
            blocks: vec![
                SaveBlock::new(
                    0,
                    &profile.header_name[..profile.header_name.len().min(15)],
                    vec![],
                )?,
                SaveBlock::new(1, b"INFO", info)?,
                SaveBlock::new(2, b"GAMESTATUS", vec![0; 7056])?,
                SaveBlock::new(3, b"MINICG", mini)?,
                SaveBlock::new(4, b"GAMEVAR", vec![0; 4096])?,
                SaveBlock::new(5, b"EFFECT", vec![0; 16384])?,
                SaveBlock::new(6, b"EFFECTFILENAME", vec![0; 4096])?,
                SaveBlock::new(7, b"MESSAGE", vec![0; 16384])?,
            ],
        };
        for (i, value) in stamp.0.into_iter().enumerate() {
            file.replace_status_word(8 + i, value)?;
        }
        // Original4da1xx clears GAMESTATUS before53ad20 and increments21 once.
        // Fields for all live actors/VM/effects/audio are filled by their own
        // producers before encode; this builder is never published by itself.
        file.replace_status_word(21, 1)?;
        Ok(file)
    }
}
