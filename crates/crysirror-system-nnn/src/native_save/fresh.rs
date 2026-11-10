// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Fresh legacy envelope production. Every native Save* begins with memset;
//! unused padding is initialized instead of depending on previous heap bytes.
use super::*;
use crate::{native_system_file::SystemStamp, native_thumbnail::NativeThumbnail};
impl LegacySaveProfile {
    pub(crate) fn validate_live_actors(&self) -> Result<()> {
        ensure!(
            self.extension_sizes.len() <= super::MAX_EXTENSIONS,
            "NNN native extension count budget"
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
        // This initializes the envelope only. AuxiliaryState writes retained
        // Omake/Cutin state and the reviewed manual-extension fallback later.
        profile.validate_live_actors()?;
        ensure!(
            thumbnail.size.iter().all(|n| *n > 0)
                && u64::from(thumbnail.size[0]) * u64::from(thumbnail.size[1]) * 4
                    == thumbnail.bgrx.len() as u64,
            "NNN live MINICG layout"
        );
        let mut info = vec![0; 32];
        let blocks = 8
            + usize::from(profile.omake)
            + usize::from(profile.cutin)
            + profile.extension_sizes.len();
        info[0..4].copy_from_slice(&(blocks as u32).to_le_bytes());
        info[4..8].copy_from_slice(&profile.version.to_le_bytes());
        info[8..12].copy_from_slice(&(profile.extension_sizes.len() as u32).to_le_bytes());
        info[12..16].copy_from_slice(&u32::from(profile.omake).to_le_bytes());
        info[16..20].copy_from_slice(&u32::from(profile.cutin).to_le_bytes());
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
        if profile.omake {
            file.blocks
                .push(SaveBlock::new(8, b"OMAKECLASS", vec![0; 12800])?);
        }
        if profile.cutin {
            file.blocks
                .push(SaveBlock::new(9, b"CUTIN", vec![0; 1024])?);
        }
        for (i, size) in profile.extension_sizes.iter().enumerate() {
            file.blocks.push(SaveBlock::new(
                100 + i as u32,
                format!("EXT{}", i + 1).as_bytes(),
                vec![0; *size as usize],
            )?);
        }
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

impl NativeSaveFile {
    /// Native GAMESTATUS: miniature comment at word440; four 256-byte comments
    /// at word1516. Only live comment policy supplies these fields.
    pub(crate) fn replace_comments(&mut self, comments: &[Vec<u8>; 4]) -> Result<()> {
        let status = self
            .blocks
            .get_mut(2)
            .context("NNN comment status absent")?;
        ensure!(status.payload.len() == 7056, "NNN comment status ABI");
        for (index, comment) in comments.iter().enumerate() {
            ensure!(!comment.contains(&0), "NNN live comment contains NUL");
            let length = comment.len().min(254);
            let at = (1516 - 8) * 4 + index * 256;
            status.payload[at..at + 256].fill(0);
            status.payload[at..at + length].copy_from_slice(&comment[..length]);
        }
        let at = (440 - 8) * 4;
        let length = comments[0].len().min(30);
        status.payload[at..at + 32].fill(0);
        status.payload[at..at + length].copy_from_slice(&comments[0][..length]);
        Ok(())
    }
}
