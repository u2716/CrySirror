// SPDX-License-Identifier: MPL-2.0
//! PC 2004 settings block at +1440, and menu writes from 0x430680.

use super::Scene;
use crysirror_resource::private_profile::PrivateProfile;

// Index is the syscall key. Values are DWORD indices relative to +1440.
// Priority, face and right-click mode live outside this block.
const FIELDS: [usize; 32] = [
    0, 3, 4, 1, 2, 0, 0, 32, 34, 33, 27, 28, 29, 30, 31, 0, 8, 11, 9, 10, 12, 6, 7, 17, 21, 18, 19,
    0, 37, 35, 36, 39,
];

impl Scene {
    pub(crate) fn setting(&self, key: u32) -> Option<u32> {
        match key {
            6 => Some(self.priority),
            15 => Some(1),
            27 => Some(u32::from(self.right_mouse_mode)),
            999 => Some(u32::from(self.settings[5] != 0)),
            1 | 3..=5 | 7..=9 | 16 | 21..=24 => {
                Some(u32::from(self.settings[FIELDS[key as usize]] != 0))
            }
            1..=31 => Some(self.settings[FIELDS[key as usize]]),
            _ => None,
        }
    }

    pub(crate) fn write_setting(&mut self, key: u32, value: u32, text: &[u8]) {
        let signed = value as i32;
        let flag = u32::from(value != 0);
        let setting = match key {
            3 | 4 if signed < 0 => return,
            1 | 3..=5 | 7..=9 | 16 | 21 | 23 | 24 | 30 => flag,
            2 => signed.max(0) as u32 & 1,
            6 => {
                self.priority = signed.max(0) as u32 % 3;
                return;
            }
            10..=14 => signed.clamp(10, 100) as u32,
            15 => {
                self.font_name = text.to_vec();
                return;
            }
            17 | 19 => signed.clamp(0, 4) as u32,
            18 | 20 | 31 => signed.clamp(0, 2) as u32,
            22 => {
                self.settings[5] = 0;
                flag
            }
            25 | 26 | 28 => value,
            27 => {
                self.right_mouse_mode = flag != 0;
                return;
            }
            29 => {
                if value <= 5 {
                    value
                } else {
                    0
                }
            }
            999 => {
                self.settings[5] = flag;
                return;
            }
            _ => return,
        };
        self.settings[FIELDS[key as usize]] = setting;
    }

    /// Menu IDs are those passed by the syscall, including the shared 33010
    /// used by all three audio switches. Values are (enabled, checked).
    pub(crate) fn menu_setting(&self, key: u32) -> Vec<(u32, bool, bool)> {
        let Some(value) = self.setting(key) else {
            return vec![];
        };
        let (first, count, selected) = match key {
            2 => (33703, 2, value),
            6 => (33719, 3, value),
            10..=14 => (32960 + 10 * (key - 10), 10, 10u32.wrapping_sub(value / 10)),
            17 => (33540, 5, value),
            18 => (33716, 3, value),
            19 => (33726, 5, value),
            20 => (33549, 3, value),
            28 => (33525, 6, value),
            29 => (33519, 5, value.wrapping_sub(1)),
            31 => (33712, 3, value),
            _ => {
                let id = match key {
                    1 => 33701,
                    3 => 33506,
                    4 => 33507,
                    5 => 33513,
                    7..=9 => 33010,
                    16 => 33548,
                    21 => 33553,
                    22 => 33715,
                    30 => 33524,
                    _ => return vec![],
                };
                return vec![(id, true, value != 0)];
            }
        };
        (0..count)
            .map(|i| {
                let enabled = match key {
                    6 => i != selected,
                    28 => self.settings[38] & (1 << i) != 0,
                    29 => value != 0,
                    _ => true,
                };
                (first + i, enabled, i == selected && (key != 28 || enabled))
            })
            .collect()
    }

    /// Fields actually emitted by the native shutdown profile writer.
    pub(crate) fn store_settings(&self, profile: &mut PrivateProfile) {
        for (section, fields) in [
            (
                "SYSTEM_INFO",
                &[
                    (1, "EFFECT_USE"),
                    (2, "WAIT_TIMING"),
                    (3, "DLG_LOAD"),
                    (4, "DLG_SAVE"),
                    (5, "ACTIVE_DRAW"),
                    (27, "MOUSE_RIGHT_MODE"),
                ][..],
            ),
            (
                "MESSAGE_INFO",
                &[
                    (16, "TEXT_SADOW"),
                    (17, "TEXT_WAIT"),
                    (18, "EFFECT_TYPE"),
                    (19, "EFFECT_TIMER"),
                    (20, "VOICE_CUT_MODE"),
                    (21, "READ_SKIP"),
                    (22, "AUTO_SKIP"),
                    (24, "AUTO_VOICE_SYNC"),
                    (25, "AUTO_MODE_1"),
                    (26, "AUTO_MODE_2"),
                ][..],
            ),
            (
                "SOUND_INFO",
                &[
                    (7, "MUSIC_USE"),
                    (8, "SE_USE"),
                    (9, "VOICE_USE"),
                    (10, "MASTER_VOLUME"),
                    (11, "MUSIC_VOLUME"),
                    (12, "SE_VOLUME"),
                    (13, "VOICE_VOLUME"),
                    (14, "VIDEO_VOLUME"),
                ][..],
            ),
            (
                "MOUSE_INFO",
                &[(28, "WHEEL_MODE"), (29, "AUTO_CLEAR"), (30, "AUTO_MOVE")][..],
            ),
            ("VIDEO_INFO", &[(31, "FRAME_CTRL")][..]),
        ] {
            for &(key, name) in fields {
                if let Some(value) = self.setting(key) {
                    profile.set(section, name, &(value as i32).to_string());
                }
            }
        }
        profile.set(
            "MESSAGE_INFO",
            "FONT",
            &encoding_rs::SHIFT_JIS.decode(&self.font_name).0,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_follow_x86_bounds_and_persist_only_profile_fields() {
        let mut scene = Scene::default();
        scene.configure(b"[MOUSE_INFO]\nWHEEL_USE=2\n");
        for key in 1..=31 {
            assert!(scene.setting(key).is_some());
        }
        scene.write_setting(3, 1, b"");
        scene.write_setting(3, u32::MAX, b"");
        assert_eq!(scene.setting(3), Some(1));
        scene.write_setting(6, 5, b"");
        assert_eq!(scene.menu_setting(6)[2], (33721, false, true));
        for key in 10..=14 {
            scene.write_setting(key, u32::MAX, b"");
            assert_eq!(scene.setting(key), Some(10));
            scene.write_setting(key, 128, b"");
            assert_eq!(scene.setting(key), Some(100));
        }
        scene.write_setting(28, 1, b"");
        assert_eq!(scene.menu_setting(28)[0], (33525, false, false));
        assert_eq!(scene.menu_setting(28)[1], (33526, true, true));
        scene.write_setting(29, 6, b"");
        assert!(
            scene
                .menu_setting(29)
                .iter()
                .all(|&(_, enabled, checked)| !enabled && !checked)
        );
        scene.write_setting(999, 42, b"");
        scene.write_setting(22, 1, b"");
        assert_eq!(scene.setting(999), Some(0));
        scene.write_setting(15, 0, b"chosen");
        scene.write_setting(8, 0, b"");
        scene.write_setting(23, 1, b"");
        let mut profile = PrivateProfile::parse("[EXTRA]\nKeep=1\n");
        scene.store_settings(&mut profile);
        let mut reopened = Scene::default();
        reopened.configure(profile.to_text().as_bytes());
        assert_eq!(reopened.setting(8), Some(0));
        assert_eq!(reopened.font_name, b"chosen");
        assert_eq!(reopened.setting(23), Some(0));
        assert_eq!(profile.integer("EXTRA", "Keep", 0), 1);
    }

    #[test]
    fn clock_switch_resets_both_counters_and_wraps_like_a_dword() {
        let mut scene = Scene::default();
        scene.reset_clock(false, u32::MAX - 4);
        scene.begin_frame(7, 17);
        assert_eq!(scene.delta, 12);
        scene.reset_clock(true, 7);
        assert_eq!((scene.delta, scene.accumulated), (0, 0));
        scene.begin_frame(500, 20);
        assert_eq!((scene.delta, scene.accumulated), (20, 20));
        scene.begin_frame(501, 8);
        assert_eq!((scene.delta, scene.accumulated), (8, 28));
    }
}
