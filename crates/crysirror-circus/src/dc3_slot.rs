//! Original DC3WY76864-byte slot prefix.40B960 accepts exactly this extent;
//! 41FCC0 rebuilds shared resources from its names and120 CPU sprite records.
//! A slot is never fabricated from an empty logical VM or a DC4PH projection.
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
const BYTES: usize = crate::dc3_settings::SLOT_BYTES;
const LIVE_BASE: usize = 17704;
const SPRITES: usize = 37332 - LIVE_BASE;
const SPRITE_BYTES: usize = 468;
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(try_from = "Vec<u8>", into = "Vec<u8>")]
pub struct Slot(Vec<u8>);
impl TryFrom<Vec<u8>> for Slot {
    type Error = anyhow::Error;
    fn try_from(bytes: Vec<u8>) -> Result<Self> {
        ensure!(
            bytes.len() == BYTES,
            "DC3WY original slot requires exactly {BYTES} bytes"
        );
        Ok(Self(bytes))
    }
}
impl From<Slot> for Vec<u8> {
    fn from(slot: Slot) -> Self {
        slot.0
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Clock {
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub year_in_century: u8,
    pub second: u8,
}
impl Slot {
    pub fn bytes(&self) -> &[u8] {
        &self.0
    }
    pub fn scene(&self) -> Result<String> {
        cstring(&self.0[8..168])
    }
    pub fn decoder_pc(&self) -> u32 {
        self.dword(19908 - LIVE_BASE)
    }
    ///41FCC0 resumes fromBuf4980, independently of the savedBuf4977 decoder.
    pub fn resume_pc(&self) -> u32 {
        self.dword(19920 - LIVE_BASE)
    }
    pub fn label(&self) -> u32 {
        self.dword(19916 - LIVE_BASE)
    }
    pub fn label_metadata(&self) -> u32 {
        self.dword(19912 - LIVE_BASE)
    }
    pub fn image_name(&self, slot: u8) -> Result<String> {
        ensure!(slot < 36, "DC3WY slot image index");
        let at = 168 + usize::from(slot) * 48;
        cstring(&self.0[at..at + 48])
    }
    pub fn sprite(&self, object: u16) -> Result<&[u8]> {
        ensure!(object < 120, "DC3WY slot sprite index");
        let at = SPRITES + usize::from(object) * SPRITE_BYTES;
        Ok(&self.0[at..at + SPRITE_BYTES])
    }
    pub fn script_bytes(&self) -> &[u8] {
        &self.0[34976 - LIVE_BASE..36076 - LIVE_BASE]
    }
    pub fn script_mirror(&self) -> &[u8] {
        &self.0[36076 - LIVE_BASE..37176 - LIVE_BASE]
    }
    ///40B630 writes six compact bytes in the mirrored script-bank region:
    ///month/day/hour/minute/year%100/second. Other bytes are retained exactly.
    pub fn clock(&self) -> Clock {
        let at = 112430 - 94568;
        Clock {
            month: self.0[at],
            day: self.0[at + 1],
            hour: self.0[at + 2],
            minute: self.0[at + 3],
            year_in_century: self.0[at + 4],
            second: self.0[at + 5],
        }
    }
    pub fn patch_clock(&mut self, clock: Clock) -> Result<()> {
        ensure!(
            (1..=12).contains(&clock.month)
                && (1..=31).contains(&clock.day)
                && clock.hour < 24
                && clock.minute < 60
                && clock.second < 60
                && clock.year_in_century < 100,
            "DC3WY original slot clock range"
        );
        let at = 112430 - 94568;
        self.0[at..at + 6].copy_from_slice(&[
            clock.month,
            clock.day,
            clock.hour,
            clock.minute,
            clock.year_in_century,
            clock.second,
        ]);
        Ok(())
    }
    ///Validate only the direct41FCC0 continuation. The native forced-replay
    ///load path has separate label/history reconstruction and must not use it.
    pub fn validate_direct_resume(&self, program: &crate::Program) -> Result<()> {
        ensure!(
            program.profile == crate::Profile::Dc3WithYou,
            "DC3WY slot/script profile mismatch"
        );
        program.instruction(self.resume_pc())?;
        Ok(())
    }
    fn dword(&self, at: usize) -> u32 {
        u32::from_le_bytes(
            self.0[at..at + 4]
                .try_into()
                .expect("bounded DC3WY slot field"),
        )
    }
}
fn cstring(bytes: &[u8]) -> Result<String> {
    let end = bytes
        .iter()
        .position(|value| *value == 0)
        .context("unterminated DC3WY slot string")?;
    let (text, errors) = encoding_rs::SHIFT_JIS.decode_without_bom_handling(&bytes[..end]);
    ensure!(!errors, "invalid DC3WY slot CP932 string");
    Ok(text.into_owned())
}

///Source419EA0's semantic resource-to-primary-object mapping. These are
///native logical IDs; they are never GPU texture handles or host pointers.
pub fn resource_object(slot: u8) -> Option<u16> {
    Some(match slot {
        1 => 0,
        2 => 10,
        3 => 20,
        4 => 30,
        5 => 40,
        6 => 12,
        7 => 22,
        8 => 32,
        9 => 42,
        19 => 50,
        20 => 54,
        25 => 65,
        _ => return None,
    })
}
pub fn object_resource(object: u16) -> Option<u8> {
    (0..36).find(|slot| resource_object(*slot) == Some(object))
}
