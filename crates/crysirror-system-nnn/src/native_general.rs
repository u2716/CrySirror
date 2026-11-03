// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Original General mode-return policy. Rendering stays in shared image/layer
//! services; this module selects the original ordinary/special exit profile.
use crate::{config::Names, native_shell::NativeMode};
use anyhow::{Result, ensure};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ImmediateReturn {
    pub capture_exit: bool,
}

impl ImmediateReturn {
    pub(crate) fn validate_entry(table: &Names) -> Result<()> {
        ensure!(
            table.integer(b"enterFadeInType")?.unwrap_or(0) == 0,
            "NNN General entry fade requires its original countdown/shared exact effect"
        );
        ensure!(
            table.integer(b"enterExitVoiceFlag")?.unwrap_or(0) == 0,
            "NNN General delayed entry/exit voice requires its original trigger policy"
        );
        Ok(())
    }
    /// The zero-fade branch calls FinalExit immediately. A nonzero profile is
    /// not an instantaneous return: it requires the native countdown and exact
    /// Overrap/PrintSimpleWipe operation, so reject it before changing owners.
    pub fn read(table: &Names, target: Option<NativeMode>) -> Result<Self> {
        // NewGame uses -1, which cannot match a positive system mode. Other
        // configured mode names don't need an implemented consumer merely to
        // select the ordinary branch. The original lookup includes atoi.
        let special_number = table
            .parameter(b"exitFadeOutSpecialMode")
            .map(crate::native_mode::number)
            .transpose()?
            .unwrap_or(-2);
        let special =
            special_number >= 0 && target.is_some_and(|mode| mode as i32 == special_number);
        let (fade_key, create_key, create_default) = if special {
            (
                b"exitFadeOutTypeSpecial".as_slice(),
                b"createExitScreenFlagSpecial".as_slice(),
                0,
            )
        } else {
            (
                b"exitFadeOutType".as_slice(),
                b"createExitScreenFlag".as_slice(),
                1,
            )
        };
        ensure!(
            table.integer(fade_key)?.unwrap_or(0) == 0,
            "NNN General nonzero exit fade requires its native countdown/shared exact effect"
        );
        Ok(Self {
            // The special create flag is read only for nonzero special fades.
            // With type zero the constructor's false survives a config token.
            capture_exit: !special && table.integer(create_key)?.unwrap_or(create_default) != 0,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn names(text: &str) -> Names {
        Names::parse(text.as_bytes(), false).unwrap()
    }
    #[test]
    fn ordinary_special_and_newgame_returns_select_original_profile_before_mutation() {
        assert!(
            ImmediateReturn::read(&names(""), None)
                .unwrap()
                .capture_exit
        );
        let table = names(
            "exitFadeOutType 2 exitFadeOutSpecialMode nothing exitFadeOutTypeSpecial 0 createExitScreenFlagSpecial 1",
        );
        assert!(
            !ImmediateReturn::read(&table, Some(NativeMode::Nothing))
                .unwrap()
                .capture_exit
        );
        assert!(ImmediateReturn::read(&table, Some(NativeMode::Opening)).is_err());
        assert!(ImmediateReturn::read(&table, None).is_err());
        let numeric =
            names("exitFadeOutType 2 exitFadeOutSpecialMode +1suffix exitFadeOutTypeSpecial 0");
        assert!(
            !ImmediateReturn::read(&numeric, Some(NativeMode::Nothing))
                .unwrap()
                .capture_exit
        );
        assert!(ImmediateReturn::read(&numeric, Some(NativeMode::Title)).is_err());
        assert!(
            !ImmediateReturn::read(&names("createExitScreenFlag 0"), None)
                .unwrap()
                .capture_exit
        );
        assert!(
            ImmediateReturn::read(&names("exitFadeOutSpecialMode unknown"), None)
                .unwrap()
                .capture_exit
        );
    }
}
