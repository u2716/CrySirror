// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Original class constructors and OBJECT field application. Native data only;
//! shared resources must still be admitted before this bank becomes runnable.
use crate::{
    fes_object_values::ObjectValues,
    native_object_pose::{Image, Plane, Sound},
};
use anyhow::{ensure, Result};
#[derive(Clone, Debug, Default)]
pub(crate) struct MetadataReference {
    pub name: Vec<u8>,
    pub frame: u32,
}
impl MetadataReference {
    pub(crate) fn parse(raw: &[u8]) -> Result<Self> {
        let at = raw.iter().position(|b| *b == b':').unwrap_or(raw.len());
        ensure!(at < 128, "CatSystem2 dynamic metadata name extent");
        let tail = raw.get(at + 1..).unwrap_or_default();
        let frame = if tail.is_empty() {
            0
        } else {
            crate::native_values::integer_prefix(tail)
                .ok_or_else(|| {
                    anyhow::anyhow!("CatSystem2 dynamic metadata native integer profile")
                })?
                .1
        };
        Ok(Self {
            name: raw[..at].to_vec(),
            frame,
        })
    }
}
#[derive(Clone, Debug)]
pub(crate) struct ImageState {
    pub pose: Image,
    pub size: [i32; 2],
    pub color: u32,
    pub plane: crate::fes_references::IndexedName,
    pub draw_id: i32,
    pub mask: u32,
    pub bitmap_type: u32,
    pub path_mode: u32,
    /// 749790: source-local clipping, and 749950's alternate coordinate mode.
    pub clip: Option<[i32; 4]>,
    pub coordinate_mode: u32,
    pub key_position: [i32;2],
    pub key_position_mode:u32,
    pub tone: [u32; 3],
    pub filter: u32,
    pub blur: [u8;2],
    /// 7499B0/74A0C0 retain an immutable engine image, never a renderer pointer.
    #[cfg(feature="runtime")]
    pub reference: Option<crate::native_image_resource::Prepared>,
    pub copy_rect: Option<[i32; 4]>,
    pub object_reference:Option<u32>,
    pub size_reference: Option<MetadataReference>,
    pub position_reference: Option<MetadataReference>,
    pub slider: Option<crate::fes_button::Slider>,
    pub filename: Vec<u8>,
    pub bg_table_kind: Option<u8>,
    pub bg_table_source: Option<Vec<u8>>,
    pub bg_table: Option<crate::fes_bg_table::Table>,
    pub frame: u32,
    pub ids: [u32; 32],
    pub mapped_ids: bool,
    pub map_changed: bool,
    pub image_changed: bool,
    pub bitmap_dirty: bool,
    pub missing: Option<(Vec<u8>, u32)>,
    /// Class 6 BUTTON: the IMAGE base plus focus and press state.
    pub button: Option<crate::fes_button::ButtonState>,
}
impl Default for ImageState {
    fn default() -> Self {
        Self {
            pose: Image::default(),
            size: [-1; 2],
            color: u32::MAX,
            plane: crate::fes_references::IndexedName {
                name: Vec::new(),
                index: 0,
            },
            draw_id: -1,
            mask: 0,
            bitmap_type: 0,
            path_mode: 0,
            clip: None, coordinate_mode: 0,key_position:[0;2],key_position_mode:0, tone: [100;3], filter: 0, blur: [1;2],
            #[cfg(feature="runtime")]
            reference: None,
            copy_rect: None,object_reference:None,
            size_reference: None,
            position_reference: None,
            slider: None,
            filename: Vec::new(),
            bg_table_kind: None,
            bg_table_source: None,
            bg_table: None,
            frame: 0,
            ids: core::array::from_fn(|i| i as u32),
            mapped_ids: false,
            map_changed: true,
            image_changed: false,
            button: None,
            bitmap_dirty: false,
            missing: None,
        }
    }
}
impl ImageState {
    pub fn native_frame(&self) -> u32 {
        if self.mapped_ids {
            self.ids.get(self.frame as usize).copied().unwrap_or(0)
        } else {
            self.frame
        }
    }
    pub fn select_frame(&mut self, value: u32) {
        //749F10 rejects out-of-range selection only after an ID map is activated.
        if (!self.mapped_ids || value <= 31) && (self.frame != value || self.map_changed) {
            self.frame = value;
            self.map_changed = false;
            self.image_changed = true;
        }
    }
    fn priority(&mut self, value: i32) {
        //749170 compares stored biased priority against the raw input, just as
        //the later749420 callback does; do not normalize away this odd comparison.
        if self.pose.priority != value {
            self.pose.previous_priority = self.pose.priority;
            self.pose.priority = value.wrapping_add(20);
        }
        self.draw_id = value;
    }
    fn visible(&mut self, value: u32) {
        if self.pose.visible != value {
            self.pose.visible = value;
            self.pose.activity_count = if value != 0 {
                self.pose.activity_count.wrapping_add(1)
            } else {
                self.pose.activity_count.wrapping_sub(1)
            };
        }
    }
    fn apply(&mut self, v: &ObjectValues, index: u32, loaded: bool) -> Result<()> {
        let flags = v.word(0);
        if loaded && flags & 1 != 0 {
            self.filename = v.string(12).to_vec();
            #[cfg(feature="runtime")]
            { self.reference = None; }
            self.copy_rect = None;self.object_reference=None;
            self.image_changed = true;
        }
        if flags & 6 != 0 {
            for (i, mask) in [2, 4].into_iter().enumerate() {
                //748BC0 reads/truncates both components before missing fields retain
                //their old value; old fractional values therefore do not survive a
                //partial integer POS update.
                let old = self.pose.position[i];
                let integer = if old >= -2147483648. && old < 2147483648. {
                    old as i32
                } else {
                    i32::MIN
                };
                self.pose.position[i] = if flags & mask != 0 {
                    v.word(44 + i) as i32 as f32
                } else {
                    integer as f32
                };
            }
        }
        if flags & 0x400000 != 0 {
            self.position_reference = Some(MetadataReference::parse(v.string(352))?);
        }
        if flags & 0x800000 != 0 {
            self.size_reference = Some(MetadataReference::parse(v.string(480))?);
        }
        if flags & 0x10 != 0 {
            self.plane = crate::fes_references::IndexedName {
                name: v.string(200).to_vec(),
                index: v.word(58) as i32,
            };
        }
        if !loaded && flags & 0x100 != 0 {
            self.size = [v.word(69) as i32, v.word(70) as i32];
            self.bitmap_dirty = true;
        }
        if !loaded && flags & 0x8000 != 0 {
            self.color = v.word(85);
            self.bitmap_dirty = true;
        }
        if flags & 0x20 != 0 {
            self.pose.alpha = v.word(59);
        }
        if loaded && flags & 0x400 != 0 {
            self.select_frame(v.word(11));
        }
        if flags & 0x800 != 0 {
            let raw = v
                .word(73)
                .wrapping_add(if v.word(218) != 0 { index } else { 0 });
            self.priority(raw as i32);
        }
        if flags & 0x1000 != 0 {
            self.visible(v.word(75));
        }
        if flags & 0x40000000 != 0 {
            self.mask = v.word(217);
        }
        if loaded {
            for i in 0..32 {
                if v.word(2) & (1 << i) != 0 {
                    self.ids[i] = v.word(12 + i);
                    self.map_changed = true;
                    self.mapped_ids = true;
                }
            }
        }
        if v.word(1) & 1 != 0 {
            self.bitmap_type = v.word(220);
        }
        if loaded && v.word(1) & 8 != 0 {
            self.path_mode = v.word(223);
        }
        Ok(())
    }
}
#[derive(Clone, Debug)]
pub(crate) struct PlaneState {
    pub pose: Plane,
    pub size: [i32; 2],
    pub vram: [f32; 4],
    pub kind: u32,
    pub division: u32,
    pub size_reference: Option<MetadataReference>,
    pub position_reference: Option<MetadataReference>,
    /// Classes 21 and 37: the message buffer this Plane displays.
    pub log_string: bool,
    pub text: Option<Box<crate::fes_text::TextObject>>,
}
impl Default for PlaneState {
    fn default() -> Self {
        Self {
            log_string: false,
            pose: Plane::default(),
            size: [800, 600],
            vram: [0., 0., 0., 1.],
            kind: 1,
            division: 0,
            size_reference: None,
            position_reference: None,
            text: None,
        }
    }
}
impl PlaneState {
    fn apply(&mut self, v: &ObjectValues, index: u32, textured: bool) -> Result<()> {
        let flags = v.word(0);
        //767DB0/767F80 test mask6 before the z check. A z-only row (mask8)
        //does not update even z. Preserve that native branch, not an idealized API.
        if flags & 6 != 0 {
            for (i, mask) in [2, 4, 8].into_iter().enumerate() {
                if flags & mask != 0 {
                    self.pose.position[i] = v.word(44 + i) as i32 as f32;
                }
            }
        }
        if flags & 0x80 != 0 {
            self.pose.base = [
                v.word(47) as i32 as f32,
                v.word(48) as i32 as f32,
                v.word(49) as i32 as f32,
                0.,
            ];
        }
        if flags & 0x100 != 0 {
            self.size = [v.word(69) as i32, v.word(70) as i32];
        }
        if flags & 0x400000 != 0 {
            self.position_reference = Some(MetadataReference::parse(v.string(352))?);
        }
        if flags & 0x800000 != 0 {
            self.size_reference = Some(MetadataReference::parse(v.string(480))?);
        }
        if textured && flags & 0x200 != 0 {
            self.vram = [v.word(71) as i32 as f32, v.word(72) as i32 as f32, 0., 0.];
        }
        if flags & 0x800 != 0 {
            self.pose.priority =
                v.word(73)
                    .wrapping_add(if v.word(218) != 0 { index } else { 0 }) as i32;
        }
        if flags & 0x1000 != 0 {
            self.pose.visible = v.word(75);
        }
        if flags & 0x8000 != 0 {
            self.pose.color = v.word(85);
        }
        if flags & 0x20 != 0 {
            self.pose.color = (self.pose.color & 0x00ffffff) | (v.word(59) << 24);
        }
        if flags & 0x2000000 != 0 {
            self.kind = v.word(152);
        }
        if v.word(1) & 4 != 0 {
            self.division = v.word(222);
        }
        Ok(())
    }
}
#[derive(Clone, Debug)]
pub(crate) struct SoundState {
    pub pose: Sound,
    pub filename: Vec<u8>,
    pub channel: i32,
    pub kind: u32,
    pub subtype: u32,
    pub group: Vec<u8>,
    //74C090..74C8F0 original transport words, separate from shared voices.
    pub load_dirty: bool,
    pub crossfade: [u32; 2],
    pub passes: u32,
    pub loop_dirty: bool,
    pub playing: bool,
    pub stop_requested: bool,
    pub play_dirty: bool,
    pub pause_dirty: bool,
    pub paused: bool,
    pub force_play: u32,
    pub play_style: Option<u32>,
    pub first_play_ms: Option<u32>,
    /// Class 10 SNDTABLE: the SOUND base plus its track table.
    pub table: Option<crate::fes_sound_table::SoundTable>,
}
impl Default for SoundState {
    fn default() -> Self {
        Self {
            pose: Sound::default(),
            filename: Vec::new(),
            channel: -1,
            kind: 0,
            subtype: 0,
            group: Vec::new(),
            load_dirty: false,
            crossfade: [0; 2],
            passes: 0,
            loop_dirty: false,
            playing: false,
            stop_requested: false,
            play_dirty: false,
            pause_dirty: false,
            paused: false,
            force_play: 0,
            //The original constructor does not initialize word98; preserve
            //unknown until native play supplies it instead of inventing zero.
            play_style: None,
            first_play_ms: None,
            table: None,
        }
    }
}
#[derive(Clone, Debug)]
pub(crate) enum State {
    Base,
    CImage(ImageState),
    Image(ImageState),
    CPlane(PlaneState),
    Plane(PlaneState),
    Sound(SoundState),
}
impl State {
    pub(crate) fn motion_seed(&self)->Option<crate::motion::PoseWords>{
        match self {
            Self::Base=>None, //748570 is a real RET, not a zero-pose seed.
            Self::CImage(v)|Self::Image(v)=>Some(v.pose.seed()),
            Self::CPlane(v)|Self::Plane(v)=>Some(v.pose.seed()),
            Self::Sound(v)=>Some(v.pose.seed()),
        }
    }
    pub(crate) fn apply_motion(&mut self,visited:bool,pose:crate::motion::PoseWords){
        match self {
            Self::Base=>{}, //748640 is a real RET after the BaseUpdate phase.
            Self::CImage(v)|Self::Image(v)=>{v.pose.apply(visited,Some(pose));},
            Self::CPlane(v)|Self::Plane(v)=>{v.pose.apply(visited,Some(pose));},
            Self::Sound(v)=>{v.pose.apply(visited,Some(pose));},
        }
    }
    pub fn new(kind: u8) -> Result<Self> {
        Ok(match kind {
            0 | 5 | 35 => Self::Base,
            1 => Self::CImage(Default::default()),
            2 => Self::Image(Default::default()),
            6 => Self::Image(ImageState { button: Some(crate::fes_button::ButtonState { enabled: true, ..Default::default() }), ..Default::default() }),
            3 => Self::CPlane(Default::default()),
            4 => Self::Plane(Default::default()),
            // DIFIMAGE and SAVEPANEL draw through IMAGE; fes_save_panel adds the save data.
            25 | 11 | 12 => Self::Image(Default::default()),
            16 | 17 => Self::Image(ImageState { bg_table_kind: Some(kind), ..Default::default() }),
            // SCROLL, SCROLL2 and SLIDER: their bitmap draws through IMAGE; 38 is LOGSTRING8's plane.
            13 | 14 | 15 => Self::Image(ImageState { button: Some(crate::fes_button::ButtonState { enabled: true, ..Default::default() }), slider: Some(crate::fes_button::Slider::new(kind == 15)), ..Default::default() }),
            24 | 38 => Self::Plane(PlaneState { text: Some(Default::default()), log_string: true, ..Default::default() }),
            21 | 29 | 37 => Self::Plane(PlaneState { text: Some(Default::default()), ..Default::default() }),
            9 => Self::Sound(Default::default()),
            10 => Self::Sound(SoundState { table: Some(Default::default()), ..Default::default() }),
            _ => anyhow::bail!(
                "CatSystem2 native class {kind} still needs shared resource publication"
            ),
        })
    }
    pub fn apply(&mut self, v: &ObjectValues, index: u32) -> Result<()> {
        let mut proposed = self.clone();
        match &mut proposed {
            Self::Base => {}
            Self::CImage(image) => image.apply(v, index, false)?,
            Self::Image(image) => {
                image.apply(v, index, true)?;
                if image.bg_table_kind.is_some() && v.word(0) & 0x8000000 != 0 {
                    image.bg_table_source = Some(v.string(800).to_vec());
                }
                // 768190: the BUTTON family after the IMAGE one.
                if let Some(button) = &mut image.button {
                    let flags = v.word(0);
                    if flags & 0x2000 != 0 {
                        button.enabled = v.word(76) != 0;
                    }
                    if flags & 0x4000 != 0 {
                        button.group = v.string(308).to_vec();
                    }
                    if flags & 0x4000000 != 0 {button.navigation=[v.word(153) as i32,v.word(154) as i32,v.word(155) as i32];}
                    if flags & 0x2000000 != 0 {
                        button.kind = v.word(152);
                    }
                }
            }
            Self::CPlane(plane) => plane.apply(v, index, false)?,
            Self::Plane(plane) => plane.apply(v, index, true)?,
            Self::Sound(sound) => {
                let flags = v.word(0);
                if flags & 1 != 0 {
                    sound.filename = v.string(12).to_vec();
                    sound.load_dirty = true;
                    sound.crossfade = [0; 2]; //7682C0 calls74C370(name,0,0).
                }
                if flags & 0x10000 != 0 {
                    sound.subtype = v.word(86);
                }
                if flags & 0x20000 != 0 {
                    sound.channel = v.word(87) as i32;
                }
                if flags & 0x2000000 != 0 {
                    sound.kind = v.word(152);
                }
                if flags & 0x10000000 != 0 {
                    sound.group = v.string(624).to_vec();
                }
                // 768340: SNDTABLE adds SIZE (word 69) to the SOUND family.
                if flags & 0x100 != 0 {
                    if let Some(table) = &mut sound.table {
                        table.resize(v.word(69))?;
                    }
                }
            }
        }
        *self = proposed;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn values(fields: &[(u8, &[u8])], target: i32) -> ObjectValues {
        let mut vm = crate::native_interpreter::Interpreter::new(1024).unwrap();
        let strings = crate::native_values::StringVariables::default();
        let mut reference = |raw: &[u8], vm: &mut crate::native_interpreter::Interpreter| {
            crate::fes_references::evaluate(raw, vm, &strings, &|_| Some(4))
        };
        crate::fes_object_values::project(
            &fields
                .iter()
                .map(|(k, v)| (*k, v.to_vec()))
                .collect::<Vec<_>>(),
            target,
            &mut vm,
            &mut reference,
            &mut |_| Ok(None),
        )
        .unwrap()
    }
    #[test]
    fn reviewed_class_constructors_and_native_partial_row_policies() {
        let mut image = State::new(1).unwrap();
        image
            .apply(
                &values(
                    &[
                        (34, b"3,4"),
                        (39, b"10,12"),
                        (46, b"0xff102030"),
                        (41, b"7"),
                        (43, b"2"),
                    ],
                    -1,
                ),
                2,
            )
            .unwrap();
        let State::CImage(image) = image else {
            panic!()
        };
        assert_eq!(image.pose.position, [3., 4.]);
        assert_eq!(image.size, [10, 12]);
        assert_eq!(image.color, 0xff102030);
        assert_eq!(image.draw_id, 9);
        assert_eq!(image.pose.priority, 29);
        assert_eq!(image.pose.visible, 2);
        assert_eq!(image.pose.activity_count, 1);
        let mut image = State::CImage(image);
        if let State::CImage(inner) = &mut image {
            inner.pose.position = [3.9, -4.9];
        }
        image.apply(&values(&[(34, b"8")], 0), 0).unwrap();
        let State::CImage(image) = image else {
            panic!()
        };
        assert_eq!(image.pose.position, [8., -4.]);
        let mut plane = State::new(4).unwrap();
        plane
            .apply(
                &values(
                    &[
                        (46, b"0xff123456"),
                        (36, b"128"),
                        (40, b"12,34"),
                        (41, b"6000"),
                    ],
                    0,
                ),
                0,
            )
            .unwrap();
        let State::Plane(plane) = plane else { panic!() };
        assert_eq!(plane.pose.color, 0x80123456);
        assert_eq!(plane.vram, [12., 34., 0., 0.]);
        assert_eq!(plane.pose.priority, 6000);
        assert_eq!(plane.size, [800, 600]);
        assert!(State::new(23).is_err());
    }
    #[test]
    fn original_image_id_map_selection_and_ignored_class_fields_are_not_genericized() {
        let mut image = State::new(2).unwrap();
        image
            .apply(
                &values(
                    &[
                        (0, b"logo"),
                        (1, b"3"),
                        (5, b"77"),
                        (39, b"9,10"),
                        (46, b"12"),
                    ],
                    0,
                ),
                0,
            )
            .unwrap();
        let State::Image(mut image) = image else {
            panic!()
        };
        assert_eq!(image.frame, 3);
        assert_eq!(image.native_frame(), 77);
        assert_eq!(image.filename, b"logo");
        assert_eq!(image.size, [-1, -1]);
        assert_eq!(image.color, u32::MAX);
        assert!(image.mapped_ids);
        image.select_frame(32);
        assert_eq!(image.frame, 3);
        image.select_frame(2);
        assert_eq!(image.native_frame(), 2);
        let mut sound = State::new(9).unwrap();
        sound
            .apply(
                &values(
                    &[
                        (0, b"voice01"),
                        (47, b"2"),
                        (48, b"-1"),
                        (55, b"1"),
                        (58, b"voice"),
                        (36, b"32"),
                    ],
                    0,
                ),
                0,
            )
            .unwrap();
        let State::Sound(sound) = sound else { panic!() };
        assert_eq!(sound.filename, b"voice01");
        assert_eq!(sound.subtype, 2);
        assert_eq!(sound.channel, -1);
        assert_eq!(sound.kind, 1);
        assert_eq!(sound.group, b"voice");
        assert_eq!(sound.pose.volume, 100);
    }
}
