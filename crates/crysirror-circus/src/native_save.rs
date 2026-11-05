//! Original DC4PH slot-body ABI. This codec preserves every imported byte;
//! it is not a live-State projection, slot restoration or fake autosave writer.
//! Sources: owner-authorized static 40C360/40C620/41DD10/444770/444BC0;
//! executable SHA256 and analysis provenance are retained in SOURCE.json.
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, fmt, ops::Range};
/// Native `_read` prefix. Original availability ignores trailing file data.
pub const BODY_BYTES: usize = 0x56fe4;
pub const LIVE_BASE: usize = 19328;
pub const IMAGE_NAME_BYTES: usize = 48;
pub const IMAGE_NAME_COUNT: usize = 51;
pub const SPRITE_BYTES: usize = 960;
pub const SPRITE_COUNT: usize = 150;
pub const MOVIE_BYTES: usize = 1668;
pub const READ_RUN_BYTES: usize = 14;
pub const READ_RUN_COUNT: usize = 400;
const IMAGE_NAMES: usize = 19496 - LIVE_BASE;
const SPRITES: usize = 121248 - LIVE_BASE;
const MOVIE: usize = 266340 - LIVE_BASE;
const BYTE_BANKS: usize = 118892 - LIVE_BASE;
const NATIVE_DATE: usize = 100154;
const READ_RUNS: usize = 106240 - LIVE_BASE;
const READ_RUN_INDEX: usize = 106236 - LIVE_BASE;
/// A complete native image admitted from bytes. There is intentionally no
/// default/zero constructor and no conversion from the partial logical VM.
#[derive(Clone, PartialEq, Eq)]
pub struct NativeSaveBody {
    bytes: Box<[u8]>,
}
impl Serialize for NativeSaveBody {
    fn serialize<S: serde::Serializer>(
        &self,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        self.bytes.serialize(serializer)
    }
}
impl<'de> Deserialize<'de> for NativeSaveBody {
    fn deserialize<D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> std::result::Result<Self, D::Error> {
        let bytes = Vec::<u8>::deserialize(deserializer)?;
        Self::from_exact(&bytes).map_err(serde::de::Error::custom)
    }
}
/// Retained41DD10 mirror and its admission evidence at capture time.
/// Later live mutations cannot change this image.40C2F0 alone stamps the
/// retained mirror's six civil-time bytes before an original slot write.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CapturedNativeBody {
    body: NativeSaveBody,
    gaps: BTreeSet<String>,
}
impl CapturedNativeBody {
    pub(crate) fn capture(body: NativeSaveBody, gaps: BTreeSet<String>) -> Self {
        Self { body, gaps }
    }
    pub(crate) fn observed_body(&self) -> &NativeSaveBody {
        &self.body
    }
    pub fn projection_gaps(&self) -> &BTreeSet<String> {
        &self.gaps
    }
    pub fn admitted_body(&self) -> Result<&NativeSaveBody> {
        ensure!(
            self.gaps.is_empty(),
            "Circus captured native save projection gaps: {:?}",
            self.gaps
        );
        Ok(&self.body)
    }
    pub(crate) fn stamp_civil_time(&mut self, time: CivilFields) -> Result<()> {
        self.body.set_civil_time_fields(
            time.year,
            time.month,
            time.day,
            time.hour,
            time.minute,
            time.second,
        )
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SlotKind {
    Normal,
    Pre,
}
impl SlotKind {
    pub fn filename(self, slot: i32) -> String {
        format!(
            "{}DC4PHDL{slot:02}.dat",
            if self == Self::Pre { "PRE" } else { "" }
        )
    }
    pub fn needs_civil_time(self, slot: i32) -> bool {
        self == Self::Pre || slot < 110
    }
    pub fn needs_thumbnail(self, slot: i32) -> bool {
        self == Self::Normal && slot != 9999
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CivilFields {
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
}
///425640 restoration policy derived from an authoritative original slot body.
/// Resource workers consume names and copied records; native texture integers
/// remain inert data and are never converted into engine/device identities.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RestorePlan {
    pub body: NativeSaveBody,
    pub scene: String,
    pub resume_pc: u32,
    pub origin: RestoreOrigin,
    pub images: Vec<RestoreImage>,
    pub audio: Vec<RestoreAudio>,
    pub prefixes: [String; 2],
    pub falling: [u32; 6],
    pub movie: Vec<u8>,
    pub restore_fade_ticks: Option<i32>,
    ///Case24 allows66511 history replay;4260D0 quickload always425640.
    pub history_replay_allowed:bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RestoreOrigin {
    Slot,
    Rollback { preserve_effects: bool },
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RestoreImage {
    pub slot: u8,
    pub name: String,
    pub native_sprite: Option<usize>,
    pub packed: bool,
    pub native_ui: bool,
}
impl RestoreImage {
    pub(crate) fn preloaded_archive_bitmap_reload(&self)->bool {
        //3C33..36 stores an archive filename without _p,.425640 does not
        //restart these packs: it passes the saved name to422CF0's bitmap
        //loader, unlike a mapped object's explicit _p, descriptor.
        (33..=36).contains(&self.slot)&&!self.packed&&self.name.to_ascii_lowercase().ends_with(".pck")
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RestoreAudio {
    pub kind: u8,
    pub name: String,
    pub looping: bool,
    /// Current native40DD40 preferences captured before async publication.
    pub voice_volume:Option<(i32,i32)>,
}
impl RestorePlan {
    pub fn decode(body: NativeSaveBody, origin: RestoreOrigin) -> Result<Self> {
        let scene = body.scene_name()?;
        ensure!(
            !scene.is_empty() && !scene.contains(['/', '\\', ':']),
            "Circus restored native scene basename"
        );
        let mut images = Vec::new();
        //425640 loads exactly0..36.37..44 are existing system resources;
        //45..50 remain retained original data, not newly loaded image slots.
        for slot in 0..37u8 {
            let name = body.native_string(19496 + usize::from(slot) * 48, 48)?;
            if !name.is_empty() {
                images.push(RestoreImage {
                    slot,
                    packed: name.get(..3).is_some_and(|s| s.eq_ignore_ascii_case("_p,")),
                    native_ui: (14..=18).contains(&slot),
                    native_sprite: crate::native_live::sprite_for_slot(slot),
                    name,
                });
            }
        }
        let mut audio = Vec::new();
        if origin == RestoreOrigin::Slot {
            for (kind, absolute, capacity, looping) in
                [(0, 21944, 32, true), (2, 265392, 128, false)]
            {
                let name = body.native_string(absolute, capacity)?;
                if !name.is_empty() {
                    audio.push(RestoreAudio {
                        voice_volume:None,
                        kind,
                        name,
                        looping,
                    });
                }
            }
        }
        for n in 0..4 {
            let looping = body.native_word(22112 + n * 4)? != 0;
            if origin == RestoreOrigin::Slot || looping {
                let name = body.native_string(21984 + n * 32, 32)?;
                if !name.is_empty() {
                    audio.push(RestoreAudio {
                        voice_volume:None,
                        kind: 1,
                        name,
                        looping,
                    });
                }
            }
        }
        let prefixes = [
            body.native_string(265812, 36)?,
            body.native_string(265552, 260)?,
        ];
        let falling = [265348, 265352, 265356, 265360, 265364, 265368]
            .map(|at| body.native_word(at))
            .into_iter()
            .collect::<Result<Vec<_>>>()?
            .try_into()
            .unwrap();
        let movie = body.movie_record().to_vec();
        let resume_pc = body.pc_fields().checkpoint_resume;
        Ok(Self {
            body,
            scene,
            resume_pc,
            origin,
            images,
            audio,
            prefixes,
            falling,
            movie,
            restore_fade_ticks: None,
            history_replay_allowed:true,
        })
    }
}
impl fmt::Debug for NativeSaveBody {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NativeSaveBody")
            .field("bytes", &self.bytes.len())
            .finish_non_exhaustive()
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NativePcFields {
    pub decode_next: u32,
    pub label_metadata: u32,
    pub label: u32,
    pub checkpoint_resume: u32,
}
/// The seven original WORDs copied by4036D0. The last three are retained
/// native metadata, not reconstructed from the logical read-range set.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NativeReadRun {
    pub scene_index: u16,
    pub metadata_and_flags: u16,
    pub first_label: u16,
    pub last_label: u16,
    pub source_word_55936: u16,
    pub source_word_55930: u16,
    pub source_word_55934: u16,
}
impl NativeReadRun {
    fn words(self) -> [u16; 7] {
        [
            self.scene_index,
            self.metadata_and_flags,
            self.first_label,
            self.last_label,
            self.source_word_55936,
            self.source_word_55930,
            self.source_word_55934,
        ]
    }
}
/// Original copied record bytes, not a GPU handle or executable pointer.
pub struct NativeSprite<'a> {
    bytes: &'a [u8],
}
impl NativeSprite<'_> {
    pub fn bytes(&self) -> &[u8] {
        self.bytes
    }
    pub fn position(&self) -> [f32; 2] {
        [float(self.bytes, 0), float(self.bytes, 4)]
    }
    /// Native4441A0 stores percentages at object+128/+132; the saved image
    /// starts at object+8. Do not convert these to normalized engine scales.
    pub fn scale_percent(&self) -> [f32; 2] {
        [float(self.bytes, 120), float(self.bytes, 124)]
    }
    /// Native444230 local pivot; no world-space/UI coordinate reinterpretation.
    pub fn pivot(&self) -> [f32; 2] {
        [float(self.bytes, 144), float(self.bytes, 148)]
    }
    /// Snapshot41DD10 explicitly zeroes this native texture identity. It is
    /// merely imported integer data and must never be dereferenced by a host.
    pub fn transient_texture_identity(&self) -> u32 {
        word(self.bytes, 180)
    }
}
fn word(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap())
}
fn float(bytes: &[u8], at: usize) -> f32 {
    f32::from_bits(word(bytes, at))
}
fn checked_record(start: usize, size: usize, count: usize, index: usize) -> Result<Range<usize>> {
    ensure!(index < count, "native save record index");
    let at = start
        .checked_add(
            index
                .checked_mul(size)
                .context("native save record overflow")?,
        )
        .context("native save record overflow")?;
    Ok(at..at + size)
}
impl NativeSaveBody {
    ///40C360/40C400 write the already captured mirror. Date and PRE-label
    /// fixups apply to this file copy only; they never recapture a later State.
    pub fn slot_bytes(
        &self,
        kind: SlotKind,
        slot: i32,
        time: Option<CivilFields>,
    ) -> Result<Vec<u8>> {
        let mut image = self.clone();
        if kind.needs_civil_time(slot) {
            let t = time.context("native Circus slot write requires sampled civil time")?;
            image.set_civil_time_fields(t.year, t.month, t.day, t.hour, t.minute, t.second)?;
        }
        if kind == SlotKind::Pre {
            let mut pc = image.pc_fields();
            pc.label = pc.label.wrapping_sub(1);
            image.set_pc_fields(pc);
        }
        Ok(image.bytes.into_vec())
    }
    pub fn scene_name(&self) -> Result<String> {
        self.native_string(19336, 128)
    }
    pub(crate) fn native_string(&self, absolute: usize, capacity: usize) -> Result<String> {
        let at = absolute
            .checked_sub(LIVE_BASE)
            .context("native string outside save body")?;
        let bytes = self
            .bytes
            .get(at..at + capacity)
            .context("native string outside save body")?;
        let end = bytes
            .iter()
            .position(|b| *b == 0)
            .context("unterminated native save string")?;
        let (text, error) = encoding_rs::SHIFT_JIS.decode_without_bom_handling(&bytes[..end]);
        ensure!(!error, "invalid native save Shift-JIS string");
        Ok(text.into_owned())
    }
    ///406140(1) +406DB0(target,2,1), followed by the caller's bank
    ///restore. The replay target is a label; it is never a bytecode offset.
    pub(crate) fn prepare_history_load(&mut self, scene_index: i32) -> Result<(i32, i32)> {
        let index = self.read_run_index() as usize;
        let run = self.read_run(index)?;
        let target = (self.pc_fields().label as i32).wrapping_add(1);
        let mut limit = if scene_index == -1 { 2 } else { i32::from(self.read_run(0)?.first_label) };
        if scene_index != -1 {
            for n in 0..=index {
                let row = self.read_run(n)?;
                if i32::from(row.scene_index as i16) == scene_index {
                    limit = i32::from(row.first_label);
                }
            }
        }
        //402A90 resets both banks, then case24 restores only118892.
        //The retained second mirror must still reflect the recorded history.
        if self.byte_bank(9, 7)? == 0 {
            let count = usize::from(run.metadata_and_flags & 0x7fff);
            ensure!(count <= 1000, "Circus history bank extent");
            self.bytes[119992 - LIVE_BASE..121092 - LIVE_BASE].fill(0);
            for n in 0..count {
                let at = 115888 - LIVE_BASE + n * 2;
                let destination = u16::from_le_bytes(self.bytes[at..at + 2].try_into()?) as usize;
                ensure!(destination < 1100, "Circus history byte-bank destination");
                self.bytes[119992 - LIVE_BASE + destination] = self.bytes[117888 - LIVE_BASE + n];
            }
            self.set_native_word(28971 * 4, count as u32)?;
        }
        for (at, value) in [
            (66511 * 4, 0),
            (27964 * 4, u32::from(run.source_word_55930)),
            (27965 * 4, u32::from(run.source_word_55930)),
            (27966 * 4, u32::from(run.source_word_55934)),
            (27967 * 4, u32::from(run.source_word_55934)),
            (27968 * 4, if index == 0 { 0 } else { u32::from(run.source_word_55936) }),
        ] { self.set_native_word(at, value)?; }
        self.set_read_run_index(index.saturating_sub(1))?;
        if index == 0 { self.bytes[READ_RUNS..READ_RUNS + 2].fill(255); }
        let mut cleared = self.read_run(index)?;
        cleared.first_label = 0;
        cleared.last_label = 0;
        self.set_read_run(index, cleared)?;
        Ok((target, limit))
    }
    ///406DB0(a3=1,a4=0): retained log directory metadata, not a slot load.
    pub(crate) fn apply_log_metadata(&mut self,snapshot:&Self,selected:usize,page_run:usize)->Result<()> {
        let run=snapshot.read_run(selected)?;
        if self.byte_bank(9,7)?==0 {
            let count=usize::from(snapshot.read_run(page_run)?.metadata_and_flags&0x7fff);
            ensure!(count<=1000,"Circus log byte-bank history extent");
            self.bytes[118892-LIVE_BASE..121092-LIVE_BASE].fill(0);
            for n in 0..count {
                let at=115888-LIVE_BASE+n*2;
                let destination=u16::from_le_bytes(self.bytes[at..at+2].try_into()?) as usize;
                ensure!(destination<1100,"Circus log byte-bank history destination");
                let value=self.bytes[117888-LIVE_BASE+n];
                self.bytes[118892-LIVE_BASE+destination]=value;
                self.bytes[119992-LIVE_BASE+destination]=value;
            }
            self.set_native_word(28971*4,count as u32)?;
        }
        for (address,value) in [(27964*4,u32::from(run.source_word_55930)),
            (27965*4,u32::from(run.source_word_55930)),(27966*4,u32::from(run.source_word_55934)),
            (27967*4,u32::from(run.source_word_55934))] {self.set_native_word(address,value)?;}
        if page_run!=0&&selected!=0 {
            self.set_read_run_index(selected-1)?;
            self.set_native_word(27968*4,u32::from(self.read_run(selected)?.source_word_55936))?;
        } else {
            self.set_read_run_index(0)?;
            self.bytes[READ_RUNS..READ_RUNS+2].fill(255);
            self.set_native_word(27968*4,0)?;
        }
        self.set_native_word(66515*4,0)?;
        Ok(())
    }
    pub(crate) fn set_native_word(&mut self, absolute: usize, value: u32) -> Result<()> {
        let at = absolute.checked_sub(LIVE_BASE).context("native word outside save body")?;
        self.bytes.get_mut(at..at + 4).context("native word outside save body")?
            .copy_from_slice(&value.to_le_bytes());
        Ok(())
    }
    pub(crate) fn native_word(&self, absolute: usize) -> Result<u32> {
        let at = absolute
            .checked_sub(LIVE_BASE)
            .context("native word outside save body")?;
        Ok(u32::from_le_bytes(
            self.bytes
                .get(at..at + 4)
                .context("native word outside save body")?
                .try_into()?,
        ))
    }
    /// Admit exactly the native body without discarding or fabricating bytes.
    pub fn from_exact(bytes: &[u8]) -> Result<Self> {
        ensure!(
            bytes.len() == BODY_BYTES,
            "native Circus body must be exactly {BODY_BYTES} bytes"
        );
        Ok(Self {
            bytes: bytes.into(),
        })
    }
    /// Match40C620's exact-prefix read, not a claim of semantic restoration.
    pub fn from_read_prefix(bytes: &[u8]) -> Result<Self> {
        Self::from_exact(
            bytes
                .get(..BODY_BYTES)
                .context("native Circus save prefix truncated")?,
        )
    }
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn pc_fields(&self) -> NativePcFields {
        NativePcFields {
            decode_next: word(&self.bytes, 22132 - LIVE_BASE),
            label_metadata: word(&self.bytes, 22136 - LIVE_BASE),
            label: word(&self.bytes, 22140 - LIVE_BASE),
            checkpoint_resume: word(&self.bytes, 22144 - LIVE_BASE),
        }
    }
    pub fn set_pc_fields(&mut self, pc: NativePcFields) {
        for (at, value) in [
            (22132, pc.decode_next),
            (22136, pc.label_metadata),
            (22140, pc.label),
            (22144, pc.checkpoint_resume),
        ] {
            self.bytes[at - LIVE_BASE..at - LIVE_BASE + 4].copy_from_slice(&value.to_le_bytes());
        }
    }
    pub fn byte_bank(&self, bank: u8, index: u8) -> Result<u8> {
        ensure!(bank < 11 && index < 100, "native Circus byte bank index");
        Ok(self.bytes[BYTE_BANKS + usize::from(bank) * 100 + usize::from(index)])
    }
    pub fn set_byte_bank(&mut self, bank: u8, index: u8, value: u8) -> Result<()> {
        ensure!(bank < 11 && index < 100, "native Circus byte bank index");
        self.bytes[BYTE_BANKS + usize::from(bank) * 100 + usize::from(index)] = value;
        Ok(())
    }
    pub fn image_name_bytes(&self, slot: usize) -> Result<&[u8]> {
        let range = checked_record(IMAGE_NAMES, IMAGE_NAME_BYTES, IMAGE_NAME_COUNT, slot)?;
        let record = &self.bytes[range];
        let end = record
            .iter()
            .position(|b| *b == 0)
            .context("unterminated native Circus image name")?;
        Ok(&record[..end])
    }
    /// Native strcpy/clear changes bytes only through the terminator. Old
    /// trailing storage is retained, including bytes unknown to this reader.
    pub fn set_image_name_bytes(&mut self, slot: usize, name: &[u8]) -> Result<()> {
        let range = checked_record(IMAGE_NAMES, IMAGE_NAME_BYTES, IMAGE_NAME_COUNT, slot)?;
        ensure!(
            name.len() < IMAGE_NAME_BYTES && !name.contains(&0),
            "native Circus image name extent"
        );
        let record = &mut self.bytes[range];
        record[..name.len()].copy_from_slice(name);
        record[name.len()] = 0;
        Ok(())
    }
    pub fn sprite(&self, index: usize) -> Result<NativeSprite<'_>> {
        Ok(NativeSprite {
            bytes: &self.bytes[checked_record(SPRITES, SPRITE_BYTES, SPRITE_COUNT, index)?],
        })
    }
    /// Original41DD10's capture fixup only. Source resources still require a
    /// proper native projection and actual shared owner-completed capture.
    pub fn clear_captured_texture_identity(&mut self, index: usize) -> Result<()> {
        let range = checked_record(SPRITES, SPRITE_BYTES, SPRITE_COUNT, index)?;
        self.bytes[range.start + 180..range.start + 184].fill(0);
        Ok(())
    }
    /// Native41DD10 clears these five complete transient sprite records and
    /// texture identities before copying the live body to its save mirror.
    /// This operation requires an existing complete body; it cannot manufacture
    /// a live scene or certify that retained engine resources were captured.
    pub fn apply_checkpoint_sprite_fixups(&mut self) {
        for index in [11, 21, 31, 41, 51] {
            let start = SPRITES + index * SPRITE_BYTES;
            self.bytes[start..start + SPRITE_BYTES].fill(0);
        }
        for index in 0..SPRITE_COUNT {
            let start = SPRITES + index * SPRITE_BYTES + 180;
            self.bytes[start..start + 4].fill(0);
        }
    }
    pub fn read_run_index(&self) -> u32 {
        word(&self.bytes, READ_RUN_INDEX)
    }
    /// Bound only writes. Import/availability must preserve even an unknown
    /// or malformed index until a semantic restoration is explicitly attempted.
    pub fn set_read_run_index(&mut self, index: usize) -> Result<()> {
        ensure!(index < READ_RUN_COUNT, "native Circus read-run index");
        self.bytes[READ_RUN_INDEX..READ_RUN_INDEX + 4]
            .copy_from_slice(&(index as u32).to_le_bytes());
        Ok(())
    }
    pub fn read_run(&self, index: usize) -> Result<NativeReadRun> {
        let range = checked_record(READ_RUNS, READ_RUN_BYTES, READ_RUN_COUNT, index)?;
        let bytes = &self.bytes[range];
        let mut words = [0u16; 7];
        for (out, pair) in words.iter_mut().zip(bytes.chunks_exact(2)) {
            *out = u16::from_le_bytes(pair.try_into().unwrap());
        }
        Ok(NativeReadRun {
            scene_index: words[0],
            metadata_and_flags: words[1],
            first_label: words[2],
            last_label: words[3],
            source_word_55936: words[4],
            source_word_55930: words[5],
            source_word_55934: words[6],
        })
    }
    pub fn set_read_run(&mut self, index: usize, run: NativeReadRun) -> Result<()> {
        let range = checked_record(READ_RUNS, READ_RUN_BYTES, READ_RUN_COUNT, index)?;
        for (pair, value) in self.bytes[range].chunks_exact_mut(2).zip(run.words()) {
            pair.copy_from_slice(&value.to_le_bytes());
        }
        Ok(())
    }
    pub fn movie_record(&self) -> &[u8] {
        &self.bytes[MOVIE..MOVIE + MOVIE_BYTES]
    }
    /// Pack already-obtained original civil-time fields. Obtaining local wall
    /// time is a host capability; this codec does not read OS or VM clocks.
    pub fn set_civil_time_fields(
        &mut self,
        year: u16,
        month: u8,
        day: u8,
        hour: u8,
        minute: u8,
        second: u8,
    ) -> Result<()> {
        ensure!(
            (1..=12).contains(&month)
                && (1..=31).contains(&day)
                && hour < 24
                && minute < 60
                && second < 60,
            "native Circus civil time fields"
        );
        self.bytes[NATIVE_DATE..NATIVE_DATE + 6].copy_from_slice(&[
            month,
            day,
            hour,
            minute,
            (year % 100) as u8,
            second,
        ]);
        Ok(())
    }
    pub fn civil_time_bytes(&self) -> [u8; 6] {
        self.bytes[NATIVE_DATE..NATIVE_DATE + 6].try_into().unwrap()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn patterned() -> Vec<u8> {
        (0..BODY_BYTES)
            .map(|i| ((i * 37 + 19) % 251) as u8)
            .collect()
    }
    #[test]
    fn exact_native_prefix_is_lossless_and_unknown_regions_survive() {
        let bytes = patterned();
        let body = NativeSaveBody::from_exact(&bytes).unwrap();
        assert_eq!(body.bytes(), bytes);
        let mut with_tail = bytes.clone();
        with_tail.extend(b"unread original trailer");
        assert_eq!(NativeSaveBody::from_read_prefix(&with_tail).unwrap(), body);
        assert!(NativeSaveBody::from_exact(&with_tail).is_err());
        assert!(NativeSaveBody::from_exact(&bytes[..BODY_BYTES - 1]).is_err());
        let mut patched = body.clone();
        patched.set_byte_bank(10, 99, 0xee).unwrap();
        let changed = patched
            .bytes()
            .iter()
            .zip(body.bytes())
            .enumerate()
            .filter_map(|(i, (a, b))| (a != b).then_some(i))
            .collect::<Vec<_>>();
        assert_eq!(changed, [BYTE_BANKS + 1099]);
        let before = patched.clone();
        assert!(patched.set_byte_bank(11, 0, 1).is_err());
        assert_eq!(patched, before);
    }
    #[test]
    fn native_pc_names_and_capture_fixups_preserve_unrelated_bytes() {
        let mut body = NativeSaveBody::from_exact(&patterned()).unwrap();
        let before = body.clone();
        let pc = NativePcFields {
            decode_next: 0x83e,
            label_metadata: 17,
            label: 3,
            checkpoint_resume: 0x831,
        };
        body.set_pc_fields(pc);
        assert_eq!(body.pc_fields(), pc);
        body.set_image_name_bytes(50, b"BG\\room").unwrap();
        assert_eq!(body.image_name_bytes(50).unwrap(), b"BG\\room");
        let range = checked_record(IMAGE_NAMES, 48, 51, 50).unwrap();
        assert_eq!(
            &body.bytes()[range.start + 8..range.end],
            &before.bytes()[range.start + 8..range.end]
        );
        let before_name = body.clone();
        assert!(body.set_image_name_bytes(51, b"x").is_err());
        assert!(body.set_image_name_bytes(0, &[b'x'; 48]).is_err());
        assert_eq!(body, before_name);
        body.clear_captured_texture_identity(149).unwrap();
        assert_eq!(body.sprite(149).unwrap().transient_texture_identity(), 0);
        assert!(body.sprite(150).is_err());
        let changed = body
            .bytes()
            .iter()
            .zip(before.bytes())
            .enumerate()
            .filter_map(|(i, (a, b))| (a != b).then_some(i))
            .collect::<Vec<_>>();
        assert!(changed.iter().all(|&i| (2804..2820).contains(&i)
            || (range.start..range.start + 8).contains(&i)
            || (SPRITES + 149 * 960 + 180..SPRITES + 149 * 960 + 184).contains(&i)));
    }
    #[test]
    fn original_civil_time_order_and_raw_float_bits_are_retained() {
        let mut body = NativeSaveBody::from_exact(&patterned()).unwrap();
        let before = body.clone();
        body.set_civil_time_fields(2026, 10, 31, 6, 40, 59).unwrap();
        assert_eq!(body.civil_time_bytes(), [10, 31, 6, 40, 26, 59]);
        let changed = body
            .bytes()
            .iter()
            .zip(before.bytes())
            .enumerate()
            .filter_map(|(i, (a, b))| (a != b).then_some(i))
            .collect::<Vec<_>>();
        assert!(
            changed
                .iter()
                .all(|i| (NATIVE_DATE..NATIVE_DATE + 6).contains(i))
        );
        let before = body.clone();
        assert!(body.set_civil_time_fields(2026, 0, 1, 0, 0, 0).is_err());
        assert_eq!(body, before);
        let record = SPRITES;
        body.bytes[record..record + 4].copy_from_slice(&0x7fc01234u32.to_le_bytes());
        assert_eq!(body.sprite(0).unwrap().position()[0].to_bits(), 0x7fc01234);
    }
    #[test]
    fn checkpoint_fixups_match_all_150_native_records_only() {
        let mut body = NativeSaveBody::from_exact(&patterned()).unwrap();
        let before = body.clone();
        body.apply_checkpoint_sprite_fixups();
        for index in 0..SPRITE_COUNT {
            let sprite = body.sprite(index).unwrap();
            assert_eq!(sprite.transient_texture_identity(), 0);
            if [11, 21, 31, 41, 51].contains(&index) {
                assert!(sprite.bytes().iter().all(|b| *b == 0));
            } else {
                let old = before.sprite(index).unwrap();
                assert_eq!(&sprite.bytes()[..180], &old.bytes()[..180]);
                assert_eq!(&sprite.bytes()[184..], &old.bytes()[184..]);
            }
        }
        assert_eq!(&body.bytes()[..SPRITES], &before.bytes()[..SPRITES]);
        let end = SPRITES + SPRITE_COUNT * SPRITE_BYTES;
        assert_eq!(&body.bytes()[end..], &before.bytes()[end..]);
        let once = body.clone();
        body.apply_checkpoint_sprite_fixups();
        assert_eq!(body, once);
    }
    #[test]
    fn original_read_run_metadata_and_sentinel_survive_roundtrip() {
        let mut body = NativeSaveBody::from_exact(&patterned()).unwrap();
        let before = body.clone();
        let run = NativeReadRun {
            scene_index: u16::MAX,
            metadata_and_flags: 0x8123,
            first_label: 5,
            last_label: 7,
            source_word_55936: 11,
            source_word_55930: 13,
            source_word_55934: 17,
        };
        body.set_read_run(399, run).unwrap();
        body.set_read_run_index(399).unwrap();
        assert_eq!(body.read_run(399).unwrap(), run);
        assert_eq!(body.read_run_index(), 399);
        let record = READ_RUNS + 399 * READ_RUN_BYTES;
        assert_eq!(
            &body.bytes()[record..record + READ_RUN_BYTES],
            &[255, 255, 0x23, 0x81, 5, 0, 7, 0, 11, 0, 13, 0, 17, 0]
        );
        assert!(
            body.bytes()
                .iter()
                .zip(before.bytes())
                .enumerate()
                .all(|(i, (a, b))| a == b
                    || (record..record + READ_RUN_BYTES).contains(&i)
                    || (READ_RUN_INDEX..READ_RUN_INDEX + 4).contains(&i))
        );
        let before = body.clone();
        assert!(body.set_read_run(400, run).is_err());
        assert!(body.set_read_run_index(400).is_err());
        assert_eq!(body, before);
    }
    #[test]
    fn slot_writes_use_the_immutable_mirror_and_native_pre_label_transform() {
        let mut mirror = NativeSaveBody::from_exact(&patterned()).unwrap();
        let pc = NativePcFields {
            decode_next: 0x831,
            label_metadata: 0x901,
            label: 0,
            checkpoint_resume: 0x831,
        };
        mirror.set_pc_fields(pc);
        let original = mirror.clone();
        let time = CivilFields {
            year: 2026,
            month: 11,
            day: 4,
            hour: 8,
            minute: 9,
            second: 10,
        };
        let normal = NativeSaveBody::from_exact(
            &mirror.slot_bytes(SlotKind::Normal, 0, Some(time)).unwrap(),
        )
        .unwrap();
        assert_eq!(normal.pc_fields(), pc);
        assert_eq!(normal.civil_time_bytes(), [11, 4, 8, 9, 26, 10]);
        let pre =
            NativeSaveBody::from_exact(&mirror.slot_bytes(SlotKind::Pre, 0, Some(time)).unwrap())
                .unwrap();
        assert_eq!(
            pre.pc_fields(),
            NativePcFields {
                label: u32::MAX,
                ..pc
            }
        );
        assert_eq!(pre.civil_time_bytes(), normal.civil_time_bytes());
        assert_eq!(
            mirror, original,
            "writing must not stamp or decrement the retained mirror"
        );
        for (at, (a, b)) in pre.bytes().iter().zip(original.bytes()).enumerate() {
            assert!(
                a == b
                    || (2812..2816).contains(&at)
                    || (NATIVE_DATE..NATIVE_DATE + 6).contains(&at)
            );
        }
        assert!(mirror.slot_bytes(SlotKind::Normal, 109, None).is_err());
        assert_eq!(
            mirror.slot_bytes(SlotKind::Normal, 110, None).unwrap(),
            mirror.bytes()
        );
        assert!(mirror.slot_bytes(SlotKind::Pre, 110, None).is_err());
        assert_eq!(SlotKind::Normal.filename(-1), "DC4PHDL-1.dat");
        assert_eq!(SlotKind::Pre.filename(2), "PREDC4PHDL02.dat");
        assert!(!SlotKind::Normal.needs_thumbnail(9999));
        assert!(!SlotKind::Pre.needs_thumbnail(0));
    }
    #[test]
    fn captured_admission_evidence_and_exact_extent_survive_serialization() {
        let body = NativeSaveBody::from_exact(&patterned()).unwrap();
        let captured = CapturedNativeBody::capture(
            body.clone(),
            BTreeSet::from(["native movie tail not audited".into()]),
        );
        let encoded = serde_json::to_vec(&captured).unwrap();
        let decoded: CapturedNativeBody = serde_json::from_slice(&encoded).unwrap();
        assert!(decoded.admitted_body().is_err());
        assert_eq!(decoded.body, body);
        assert_eq!(decoded.projection_gaps(), captured.projection_gaps());
        let admitted = CapturedNativeBody::capture(body.clone(), BTreeSet::new());
        assert_eq!(admitted.admitted_body().unwrap(), &body);
        let short = serde_json::to_vec(&vec![0u8; BODY_BYTES - 1]).unwrap();
        assert!(serde_json::from_slice::<NativeSaveBody>(&short).is_err());
    }
}
