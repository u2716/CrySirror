// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Original class-specific motion callbacks, never generic Scene/Audio state.
//! Reusable transforms, composition and mixing remain shared engine services.
//! Callback output must still be published with a real engine resource receipt.
use crate::motion::PoseWords;

///The reviewed SSE CVTTSS2SI profile returns the integer-indefinite DWORD for
///NaN, infinity and overflow; Rust's saturating `as i32` alone differs.
fn x86_i32(value: f32) -> i32 {
    if value >= -2147483648. && value < 2147483648. {
        value as i32
    } else {
        i32::MIN
    }
}
fn read(pose: PoseWords, index: usize) -> f32 {
    f32::from_bits(pose.0[index])
}
fn pair_changed(old: [f32; 2], new: [f32; 2]) -> bool {
    old[0] != new[0] || old[1] != new[1]
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct ImageChange {
    pub visual: bool,
    pub priority: bool,
}
#[derive(Clone, Debug)]
pub(crate) struct Image {
    pub position: [f32; 2], //208/212, seeded into6/7, not UV coordinates
    pub offset: [f32; 2],   //216/220, not touched by this motion callback
    pub pivot: [f32; 2],    //224/228, seeded into3/4
    pub rotate: f32,
    pub scale: [f32; 2],
    pub alpha: u32,
    ///Stored priority already includes original+20. This differs from seed5.
    pub priority: i32,
    pub previous_priority: i32,
    pub visible: u32,
    pub activity_count: u32,
}
impl Default for Image {
    fn default() -> Self {
        Self {
            position: [0.; 2],
            offset: [0.; 2],
            pivot: [0.; 2],
            rotate: 0.,
            scale: [1.; 2],
            alpha: 255,
            priority: -1,
            previous_priority: -1,
            visible: 0,
            activity_count: 0,
        }
    }
}
impl Image {
    ///749330 zeros144bytes first; unused matrix/RGB words are NOT identity or
    ///white. Original6/7 position and3/4 pivot must not be exchanged.
    pub fn seed(&self) -> PoseWords {
        let mut words = [0; 36];
        for (index, value) in [
            (3, self.pivot[0]),
            (4, self.pivot[1]),
            (5, self.priority.wrapping_sub(20) as f32),
            (6, self.position[0]),
            (7, self.position[1]),
            (25, self.scale[0]),
            (26, self.scale[1]),
            (27, 1.),
            (31, self.rotate),
        ] {
            words[index] = value.to_bits();
        }
        words[28] = (self.alpha as u8 as u32) << 24;
        words[35] = self.visible;
        PoseWords(words)
    }
    ///749420 returns0 when the object is inactive or there is no initialized
    ///manager output. Caller cannot substitute a zero pose for missing output.
    pub fn apply(&mut self, active: bool, output: Option<PoseWords>) -> Option<ImageChange> {
        let pose = output.filter(|_| active)?;
        let mut change = ImageChange::default();
        let position = [read(pose, 6), read(pose, 7)];
        if pair_changed(self.position, position) {
            self.position = position;
            change.visual = true;
        }
        let alpha = pose.0[28] >> 24;
        if self.alpha != alpha {
            self.alpha = alpha;
            change.visual = true;
        }
        let scale = [read(pose, 25), read(pose, 26)];
        if pair_changed(self.scale, scale) {
            self.scale = scale;
            change.visual = true;
        }
        let rotate = read(pose, 31);
        if self.rotate != rotate {
            self.rotate = rotate;
            change.visual = true;
        }
        let pivot = [read(pose, 3), read(pose, 4)];
        if pair_changed(self.pivot, pivot) {
            self.pivot = pivot;
            change.visual = true;
        }
        let priority = x86_i32(read(pose, 5));
        //The native compares stored (biased) priority with the unbiased pose
        //value, then stores value+20. Preserve this seemingly odd comparison.
        if self.priority != priority {
            self.previous_priority = self.priority;
            self.priority = priority.wrapping_add(20);
            change.priority = true;
        }
        let visible = pose.0[35];
        if self.visible != visible {
            self.visible = visible;
            self.activity_count = if visible != 0 {
                self.activity_count.wrapping_add(1)
            } else {
                self.activity_count.wrapping_sub(1)
            };
            change.visual = true;
        }
        Some(change)
    }
}

#[derive(Clone, Debug)]
pub(crate) struct Plane {
    pub position: [f32; 4],
    pub base: [f32; 4],
    pub euler: [u32; 3],
    pub matrix: [u32; 16],
    pub scale: [f32; 4],
    pub priority: i32,
    pub color: u32,
    pub visible: u32,
}
impl Default for Plane {
    fn default() -> Self {
        let mut matrix = [0; 16];
        for i in [0, 5, 10, 15] {
            matrix[i] = 1f32.to_bits();
        }
        Self {
            position: [0., 0., 0., 1.],
            base: [0., 0., 0., 1.],
            euler: [0; 3],
            matrix,
            scale: [1.; 4],
            priority: 5000,
            color: u32::MAX,
            visible: 0,
        }
    }
}
impl Plane {
    ///POS lives at native0x30, BASE at0x40.74AC30 seeds BASE x/y
    ///into3/4 and priority into5; POS x/y/z goes into6/7/8.
    pub fn seed(&self) -> PoseWords {
        let mut words = [0; 36];
        words[3] = self.base[0].to_bits();
        words[4] = self.base[1].to_bits();
        words[5] = (self.priority as f32).to_bits();
        for i in 0..3 {
            words[6 + i] = self.position[i].to_bits();
            words[25 + i] = self.scale[i].to_bits();
            words[29 + i] = self.euler[i];
        }
        words[9..25].copy_from_slice(&self.matrix);
        words[28] = self.color;
        words[35] = self.visible;
        PoseWords(words)
    }
    ///74AD40 copies the complete matrix and records Euler words; applying Euler
    ///again as another rotation would double-transform the original output.
    ///Word5 becomes BASE.z but does NOT change stored priority.
    pub fn apply(&mut self, active: bool, output: Option<PoseWords>) -> bool {
        let Some(pose) = output.filter(|_| active) else {
            return false;
        };
        for (old, first) in [
            (&mut self.base, 3),
            (&mut self.position, 6),
            (&mut self.scale, 25),
        ] {
            let value = [
                read(pose, first),
                read(pose, first + 1),
                read(pose, first + 2),
                0.,
            ];
            if old.iter().zip(value).any(|(a, b)| *a != b) {
                *old = value;
            }
        }
        self.euler.copy_from_slice(&pose.0[29..32]);
        self.matrix.copy_from_slice(&pose.0[9..25]);
        self.color = pose.0[28];
        self.visible = pose.0[35];
        //Native always marks the original object dirty after copying color.
        true
    }
}

#[derive(Clone, Debug)]
pub(crate) struct Sound {
    pub volume: u32,
    pub pan: i32,
}
impl Default for Sound {
    fn default() -> Self {
        Self {
            volume: 100,
            pan: 0,
        }
    }
}
impl Sound {
    ///74C520 stores volume's low byte in ARGB alpha and integer pan as float6;
    ///it does not seed a generic visual matrix/scale or a PCM sample position.
    pub fn seed(&self) -> PoseWords {
        let mut words = [0; 36];
        words[6] = (self.pan as f32).to_bits();
        words[28] = (self.volume as u8 as u32) << 24;
        PoseWords(words)
    }
    pub fn apply(&mut self, active: bool, output: Option<PoseWords>) -> Option<bool> {
        let pose = output.filter(|_| active)?;
        let pan = x86_i32(read(pose, 6));
        let volume = pose.0[28] >> 24;
        let changed = self.pan != pan || self.volume != volume;
        self.pan = pan;
        self.volume = volume;
        Some(changed)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_class_seeds_are_distinct_and_no_zero_output_is_fabricated() {
        let mut image = Image::default();
        let seed = image.seed();
        assert_eq!(seed.0[5], (-21f32).to_bits());
        assert_eq!(seed.0[28], 0xff000000);
        assert_eq!(seed.0[9..25], [0; 16]);
        assert_eq!(seed.0[27], 1f32.to_bits());
        assert_eq!(image.apply(false, Some(seed)), None);
        assert_eq!(image.apply(true, None), None);
        let plane = Plane::default();
        let seed = plane.seed();
        assert_eq!(seed.0[5], 5000f32.to_bits());
        assert_eq!(seed.0[9], 1f32.to_bits());
        assert_eq!(seed.0[28], u32::MAX);
        assert_eq!(plane.position[3], 1., "native vector constant8D54A0 W is1");
        assert_eq!(plane.scale[3], 1., "native vector constant8D5690 W is1");
        let sound = Sound::default();
        assert_eq!(sound.seed().0[28], 100 << 24);
        assert_eq!(sound.seed().0[25..28], [0; 3]);
    }
    #[test]
    fn cimage_position_pivot_priority_alpha_and_visibility_follow_original_callback() {
        let mut image = Image::default();
        image.offset = [8., 9.];
        let mut pose = image.seed();
        for (i, v) in [
            (3, 11f32),
            (4, 12.),
            (5, 120.),
            (6, 21.),
            (7, 22.),
            (25, 2.),
            (26, 3.),
            (31, 0.5),
        ] {
            pose.0[i] = v.to_bits();
        }
        pose.0[28] = 0x7f123456;
        pose.0[35] = 2;
        assert_eq!(
            image.apply(true, Some(pose)),
            Some(ImageChange {
                visual: true,
                priority: true
            })
        );
        assert_eq!(image.position, [21., 22.]);
        assert_eq!(image.pivot, [11., 12.]);
        assert_eq!(image.offset, [8., 9.]);
        assert_eq!(
            (
                image.alpha,
                image.priority,
                image.previous_priority,
                image.activity_count
            ),
            (127, 140, -1, 1)
        );
        assert_eq!(
            image.seed().0[28],
            0x7f000000,
            "RGB is not copied into CIMAGE tint"
        );
        image.priority = 120;
        assert_eq!(
            image.apply(true, Some(pose)),
            Some(ImageChange {
                visual: false,
                priority: false
            })
        );
        pose.0[35] = 1;
        image.apply(true, Some(pose));
        pose.0[35] = 0;
        image.apply(true, Some(pose));
        assert_eq!(
            image.activity_count, 1,
            "native nonzero-to-nonzero visibility increments too"
        );
    }
    #[test]
    fn cplane_seed_priority_becomes_output_z_without_changing_priority_or_double_rotating() {
        let mut plane = Plane::default();
        plane.base[2] = 13.;
        let mut pose = plane.seed();
        assert_eq!(read(pose, 5), 5000.);
        pose.0[3] = 2f32.to_bits();
        pose.0[5] = 41f32.to_bits();
        pose.0[6] = 7f32.to_bits();
        pose.0[25] = 3f32.to_bits();
        pose.0[29] = 1f32.to_bits();
        pose.0[9 + 12] = 99f32.to_bits();
        pose.0[28] = 0x11223344;
        pose.0[35] = 3;
        assert!(!plane.apply(false, Some(pose)));
        assert_eq!(plane.base[2], 13.);
        assert!(plane.apply(true, Some(pose)));
        assert_eq!(plane.base, [2., 0., 41., 0.]);
        assert_eq!(plane.priority, 5000);
        assert_eq!(plane.position, [7., 0., 0., 0.]);
        assert_eq!(plane.scale, [3., 1., 1., 0.]);
        assert_eq!(plane.euler[0], 1f32.to_bits());
        assert_eq!(plane.matrix[12], 99f32.to_bits());
        assert_eq!(plane.color, 0x11223344);
        assert_eq!(plane.seed().0[5], 5000f32.to_bits());
        plane.base[0] = -0.;
        pose.0[3] = 0f32.to_bits();
        plane.base[1] = read(pose, 4);
        plane.base[2] = read(pose, 5);
        plane.apply(true, Some(pose));
        assert_eq!(
            plane.base[0].to_bits(),
            (-0f32).to_bits(),
            "equal native vectors retain existing signed-zero bits"
        );
    }
    #[test]
    fn sound_volume_low_byte_pan_truncation_and_sse_indefinite_are_native_policy() {
        let mut sound = Sound {
            volume: 300,
            pan: -7,
        };
        let mut pose = sound.seed();
        assert_eq!(pose.0[28], 44 << 24);
        assert_eq!(read(pose, 6), -7.);
        pose.0[6] = 13.75f32.to_bits();
        pose.0[28] = 0xaaffffff;
        assert_eq!(sound.apply(true, Some(pose)), Some(true));
        assert_eq!((sound.volume, sound.pan), (170, 13));
        assert_eq!(sound.apply(true, Some(pose)), Some(false));
        for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, 2147483648f32] {
            pose.0[6] = value.to_bits();
            sound.apply(true, Some(pose));
            assert_eq!(sound.pan, i32::MIN);
        }
        assert_eq!(x86_i32(-2147483648.), i32::MIN);
        assert_eq!(x86_i32(-9.9), -9);
    }
}
