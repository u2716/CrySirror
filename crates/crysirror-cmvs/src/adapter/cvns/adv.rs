// SPDX-License-Identifier: MPL-2.0
//! CVNS ADV windows (`0x420510`, 0x694 bytes): a message window with up to
//! 20 sprite parts, five bitmaps, a click-wait icon (part 1) and 12 buttons.

use crysirror_core::service::UploadedImage;

pub(super) const BITMAPS: usize = 5;
pub(super) const BUTTONS: usize = 12;

/// One state image of a button at +152 + 18 * state.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct ButtonState {
    pub bitmap: i16,
    pub crop: [i16; 4],
    /// +162/+164: offset from the window position.
    pub offset: [i16; 2],
}

/// A button record of 128 bytes at +148.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct Button {
    /// +148.
    pub enabled: bool,
    /// +152: state images.
    pub states: [ButtonState; 6],
    /// +260: the part showing the button.
    pub part: u16,
    /// +262..+268: hit rectangle, made absolute by `0x420bd0`.
    pub rect: [i16; 4],
    /// +270: current state.
    pub state: u16,
    /// +272: the state whose image the part shows.
    pub applied: u16,
    /// +274.
    pub value: u16,
}

pub(super) struct Adv {
    /// [26]: the message window index.
    pub message: usize,
    /// [30]..[33].
    pub rect: [i32; 4],
    /// [27].
    pub shown: bool,
    /// [28]: hidden by the user (`0x211b`).
    pub user_hidden: bool,
    /// [29].
    pub hide_allowed: bool,
    /// +136..+140: `0x420960` icon placement mode and offset.
    pub icon_mode: i16,
    pub icon_offset: [i16; 2],
    original_fields: Option<Box<[u8; 1580]>>,
    pub buttons: [Button; BUTTONS],
    /// +84: five bitmaps for type-30 parts.
    pub bitmaps: [Option<UploadedImage>; BITMAPS],
    pub bitmap_components: [Option<super::bitmap::Components>; BITMAPS],
}

impl Adv {
    /// `0x420510` and `0x420600`.
    pub fn new(message: usize, rect: [i32; 4]) -> Self {
        Self {
            message,
            rect,
            shown: false,
            user_hidden: false,
            hide_allowed: true,
            icon_mode: 0,
            icon_offset: [0; 2],
            original_fields: None,
            buttons: [Button::default(); BUTTONS],
            bitmaps: Default::default(),
            bitmap_components: Default::default(),
        }
    }

    /// sub_4205b0 prefixes the message owner, then copies bytes 104..1684.
    pub(super) fn record(&self, index: usize, out: &mut Vec<u8>) -> anyhow::Result<()> {
        let mut fields = self.original_fields.as_deref().copied().unwrap_or([0;1580]);
        for (i,value) in [self.message as u32,self.shown as u32,self.user_hidden as u32,
            self.hide_allowed as u32,self.rect[0] as u32,self.rect[1] as u32,
            self.rect[2] as u32,self.rect[3] as u32].into_iter().enumerate() {
            fields[4*i..4*i+4].copy_from_slice(&value.to_le_bytes());
        }
        fn set(fields:&mut [u8], at:usize, value:i16) {
            fields[at..at+2].copy_from_slice(&value.to_le_bytes());
        }
        set(&mut fields,32,self.icon_mode);
        set(&mut fields,34,self.icon_offset[0]);
        set(&mut fields,36,self.icon_offset[1]);
        for (b,raw) in self.buttons.iter().zip(fields[44..].chunks_exact_mut(128)) {
            raw[..4].copy_from_slice(&u32::from(b.enabled).to_le_bytes());
            for (state, data) in b.states.iter().zip(raw[4..112].chunks_exact_mut(18)) {
                let [x,y,w,h] = state.crop;
                let [dx,dy] = state.offset;
                for (i,value) in [state.bitmap,x,y,w,h,dx,dy].into_iter().enumerate() {
                    set(data,2*i,value);
                }
            }
            let [x,y,w,h]=b.rect;
            for (i,v) in [b.part as i16,x,y,w,h,b.state as i16,b.applied as i16,b.value as i16].into_iter().enumerate() {
                set(raw,112+2*i,v);
            }
        }
        let mut payload=(self.message as u16).to_le_bytes().to_vec();
        payload.extend_from_slice(&fields);
        crate::vm::save_slot::csp1::append(out,0x400,&[index as u16],&payload)
    }

    pub(super) fn from_record(data:&[u8]) -> anyhow::Result<Self> {
        anyhow::ensure!(data.len()==1582,"invalid CSP1 ADV record length");
        let fields: &[u8;1580] = data[2..].try_into()?;
        let word=|at:usize| i32::from_le_bytes(fields[at..at+4].try_into().unwrap());
        let short=|raw:&[u8],at:usize| i16::from_le_bytes(raw[at..at+2].try_into().unwrap());
        let owner=u16::from_le_bytes(data[..2].try_into()?);
        anyhow::ensure!(owner < 8 && i32::from(owner)==word(0), "invalid CSP1 ADV message owner");
        let mut adv=Self::new(usize::from(owner),[word(16),word(20),word(24),word(28)]);
        adv.original_fields=Some(Box::new(*fields));
        adv.shown=word(4)!=0;
        adv.user_hidden=word(8)!=0;
        adv.hide_allowed=word(12)!=0;
        adv.icon_mode=short(fields,32);
        adv.icon_offset=[short(fields,34),short(fields,36)];
        for (b,raw) in adv.buttons.iter_mut().zip(fields[44..].chunks_exact(128)) {
            b.enabled=u32::from_le_bytes(raw[..4].try_into()?)!=0;
            for (state,data) in b.states.iter_mut().zip(raw[4..112].chunks_exact(18)) {
                state.bitmap=short(data,0);
                state.crop=std::array::from_fn(|i|short(data,2+2*i));
                state.offset=[short(data,10),short(data,12)];
            }
            b.part=short(raw,112) as u16;
            b.rect=std::array::from_fn(|i|short(raw,114+2*i));
            b.state=short(raw,122) as u16;
            b.applied=short(raw,124) as u16;
            b.value=short(raw,126) as u16;
            anyhow::ensure!(!b.enabled || (b.part<20 && b.state<6), "invalid active CSP1 ADV button");
        }
        Ok(adv)
    }

    /// `0x420c50`: the last enabled button containing the point, if any.
    pub fn hit(&self, point: [i32; 2]) -> Option<usize> {
        self.buttons.iter().enumerate().rev().find_map(|(index, button)| {
            let [x, y, w, h] = button.rect.map(|value| i32::from(value as u16));
            (button.enabled && x <= point[0] && y <= point[1] && point[0] <= x + w && point[1] <= y + h)
                .then_some(index)
        })
    }
}
