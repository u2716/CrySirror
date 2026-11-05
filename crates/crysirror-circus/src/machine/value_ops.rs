//! Original40CC50/40D240 scalar banks. Audio records are preference values,
//! never audio-device pointers or a duplicate mixer implementation.
//!
//! Pinned DC4PH's full Buf-reference/argument-alias audit finds no host-policy
//! reads for99/{16,17,18,27,180,182}: these are stored/script-readable settings.
//! Their40CC50 getter and40D240 setter are the complete runtime interface;
//! reset/default and system.dat bulk copies retain the original storage. See
//! target/verification/circus-execution/native-v99-buffer-alias-proof.json.
//! Conversely200/207 have a real453310 MPG-volume consumer in movie_ops.rs.
use super::*;
use crate::settings::{SETTINGS_BYTES, SystemData};
/// Constructor40EC50 explicit assignments plus its zeroed native save body.
/// Platform-owned4 and uninitialized100/226 are deliberately absent.
pub(super) fn runtime_defaults() -> [(u8, i32); 21] {
    let mut defaults=[
        0, 2, 179, 183, 186, 187, 189, 192, 193, 195, 204, 224, 233, 234, 241, 243, 245, 246, 253,
        254,3,
    ]
    .map(|index| (index, 0));
    //43A2F0 explicitly stores1 at renderer+24 (ASM43A351). This is
    //external window-policy state, not a fabricated zeroed Buf DWORD.
    defaults[20]=(3,1);defaults
}
/// Getter40CC50 has an explicit storage/platform branch for these selectors.
/// All others reach the original zero-return default, rather than requiring a
/// fabricated backing field. Selector1 is an explicit zero-return branch.
pub(super) fn known_selector(index: i16) -> bool {
    matches!(index,0..=5|7..=19|21|26..=30|100|179..=214|217|218|221|224..=227|233|234|237|241|243|245|246|253|254)
}
///40CC50/40D240 aliases outside the preference prefix. Decoder/worker policy
///uses these same DWORD identities; keeping a second unsynchronized selector
///map would make accepted assignments invisible to their native consumers.
pub(super) fn runtime_field(index:i16)->Option<u32> {
    Some(match index {
        0=>272076,2=>272080,19=>272088,100=>272087,179=>272089,
        183=>66584,186=>272086,187=>297158,189=>297157,
        192=>272079,193=>272078,195=>272077,204=>272084,
        224=>272081,226=>272082,233=>28970,234=>66329,241=>29722,
        243=>297152,245=>297156,246=>297154,253=>297155,254=>297153,
        _=>return None,
    })
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(super) struct NativeValues {
    /// Native298128: registered per-source entries0..49. Record50 is always
    /// addressable, even when the source collection has no entries.
    source_count: u8,
}
impl NativeValues {
    pub(super) fn set_source_count(&mut self, count: usize) -> Result<()> {
        ensure!(count <= 50, "Circus native audio source count");
        self.source_count = count as u8;
        Ok(())
    }
    pub(super) fn value(&self, data: &SystemData, bank: i16, index: i16) -> Result<Option<i32>> {
        let offset = match bank {
            93 => {
                if !(0..20).contains(&index) {
                    return Ok(Some(0));
                }
                17640 + index as usize * 4
            }
            94 | 95 => {
                ensure!(index >= 0, "Circus native audio source negative index");
                if index >= i16::from(self.source_count) && index != 50 {
                    return Ok(Some(-1));
                }
                ensure!(index <= 50, "Circus native audio source extent");
                SETTINGS_BYTES + index as usize * 40 + if bank == 94 { 4 } else { 0 }
            }
            _ => return Ok(None),
        };
        Ok(Some(i32::from_le_bytes(
            data.bytes()[offset..offset + 4].try_into()?,
        )))
    }
    pub(super) fn assign(
        &self,
        data: &mut SystemData,
        bank: i16,
        index: i16,
        value: i32,
    ) -> Result<bool> {
        let offset = match bank {
            93 => {
                if !(0..20).contains(&index) {
                    return Ok(true);
                }
                17640 + index as usize * 4
            }
            94 | 95 => {
                ensure!(index >= 0, "Circus native audio source negative index");
                if index >= i16::from(self.source_count) && index != 50 {
                    return Ok(true);
                }
                ensure!(index <= 50, "Circus native audio source extent");
                SETTINGS_BYTES + index as usize * 40 + if bank == 94 { 4 } else { 0 }
            }
            _ => return Ok(false),
        };
        let mut bytes = data.bytes().to_vec();
        // Original setter tests the index, not the value, for its lower-bound
        // branch. Negative volume therefore survives; only >100 is clamped.
        let value = if bank == 94 { value.min(100) } else { value };
        bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
        *data = SystemData::try_from(bytes)?;
        Ok(true)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn inline_gallery_values_alias_preference_dwords_and_preserve_invalid_noops() {
        let values = NativeValues::default();
        let mut data = SystemData::default();
        assert!(values.assign(&mut data, 93, 19, -73).unwrap());
        assert_eq!(data.dword(4429), -73);
        assert_eq!(values.value(&data, 93, 19).unwrap(), Some(-73));
        let before = data.bytes().to_vec();
        assert!(values.assign(&mut data, 93, 20, 9).unwrap());
        assert_eq!(data.bytes(), before);
        assert_eq!(values.value(&data, 93, 20).unwrap(), Some(0));
        assert_eq!(values.value(&data, 92, 0).unwrap(), None);
    }
    #[test]
    fn special_source50_survives_empty_collection_and_volume_keeps_native_signed_behavior() {
        let mut values = NativeValues::default();
        let mut data = SystemData::default();
        assert_eq!(values.value(&data, 95, 50).unwrap(), Some(1));
        assert_eq!(values.value(&data, 94, 50).unwrap(), Some(100));
        assert_eq!(values.value(&data, 94, 0).unwrap(), Some(-1));
        values.assign(&mut data, 94, 0, 71).unwrap();
        values.set_source_count(1).unwrap();
        assert_eq!(values.value(&data, 94, 0).unwrap(), Some(0));
        values.assign(&mut data, 94, 0, -7).unwrap();
        assert_eq!(values.value(&data, 94, 0).unwrap(), Some(-7));
        values.assign(&mut data, 94, 0, 101).unwrap();
        assert_eq!(values.value(&data, 94, 0).unwrap(), Some(100));
        values.assign(&mut data, 95, 0, -92).unwrap();
        assert_eq!(values.value(&data, 95, 0).unwrap(), Some(-92));
        assert!(values.value(&data, 94, -1).is_err());
        assert!(values.set_source_count(51).is_err());
    }
}
