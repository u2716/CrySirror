// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Legacy 0x53c060/0x534b30 parameters. SYSTEM DATA file offsets differ from
//! the CSystemFile object by 68 bytes. UI controls belong to shared UI services.
use anyhow::{Result, ensure};
#[derive(Clone, Copy, Debug)]
pub struct Setting {
    pub id: i32,
    pub name: &'static str,
    pub offset: usize,
    pub maximum: i32,
}
pub const SETTINGS: &[Setting] = &[
    Setting {
        id: 105,
        name: "Message speed",
        offset: 136,
        maximum: 4,
    },
    Setting {
        id: 2,
        name: "Skip unread text",
        offset: 140,
        maximum: 1,
    },
    Setting {
        id: 6,
        name: "Automatic advance",
        offset: 164,
        maximum: 1,
    },
    Setting {
        id: 4,
        name: "Fullscreen",
        offset: 84,
        maximum: 1,
    },
    Setting {
        id: 100,
        name: "Music volume",
        offset: 176,
        maximum: 100,
    },
    Setting {
        id: 101,
        name: "Sound volume",
        offset: 184,
        maximum: 100,
    },
    Setting {
        id: 102,
        name: "Voice volume",
        offset: 180,
        maximum: 100,
    },
    Setting {
        id: 103,
        name: "Movie volume",
        offset: 188,
        maximum: 100,
    },
    Setting {
        id: 104,
        name: "Sound voice volume",
        offset: 212,
        maximum: 100,
    },
    Setting {
        id: 106,
        name: "Window transparency",
        offset: 1932,
        maximum: 100,
    },
    Setting {
        id: 200,
        name: "Music enabled",
        offset: 192,
        maximum: 1,
    },
    Setting {
        id: 201,
        name: "Sound enabled",
        offset: 200,
        maximum: 1,
    },
    Setting {
        id: 202,
        name: "Voice enabled",
        offset: 196,
        maximum: 1,
    },
    Setting {
        id: 203,
        name: "Movie enabled",
        offset: 204,
        maximum: 1,
    },
    Setting {
        id: 204,
        name: "Sound voice enabled",
        offset: 216,
        maximum: 1,
    },
];
pub fn setting(id: i32) -> Result<Setting> {
    SETTINGS
        .iter()
        .find(|s| s.id == id)
        .copied()
        .ok_or_else(|| anyhow::anyhow!("NNN unsupported settings parameter {id}"))
}
impl crate::native_system_file::NativeSystemFile {
    pub(crate) fn set_menu_parameter(&mut self, id: i32, value: i32) -> Result<()> {
        let setting = setting(id)?;
        ensure!(
            (0..=setting.maximum).contains(&value),
            "NNN settings parameter {id} range"
        );
        self.set_setting_word(setting.offset, value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native_system_file::{NativeSystemFile, SystemLimits, SystemNames, SystemStamp};
    #[test]
    fn native_settings_write_file_offsets_and_reject_bad_values_without_mutation() {
        let defaults = NativeSystemFile::defaults(
            SystemLimits::new(1, 1, 1),
            SystemStamp([2026, 11, 9, 1, 12, 0, 0, 0]),
            &SystemNames {
                font: b"fixture".to_vec(),
                surname: vec![],
                given_name: vec![],
            },
        )
        .unwrap();
        let mut file = defaults.clone();
        let original = file.encode().unwrap();
        for (id, value, offset) in [
            (100, 37, 176),
            (201, 0, 200),
            (105, 4, 136),
            (106, 53, 1932),
        ] {
            file.set_menu_parameter(id, value).unwrap();
            assert_eq!(file.system_word(offset).unwrap(), value);
        }
        let bytes = file.encode().unwrap();
        assert_eq!(&bytes[64 + 176..64 + 180], &37i32.to_le_bytes());
        assert_eq!(&bytes[64 + 1932..64 + 1936], &53i32.to_le_bytes());
        assert_eq!(&bytes[64 + 180..64 + 188], &original[64 + 180..64 + 188]);
        let restored = NativeSystemFile::load(&bytes, &defaults).unwrap();
        assert_eq!(restored.system_word(136).unwrap(), 4);
        assert!(file.set_menu_parameter(105, 5).is_err());
        assert!(file.set_menu_parameter(999, 0).is_err());
        assert_eq!(file.encode().unwrap(), bytes);
    }
}
