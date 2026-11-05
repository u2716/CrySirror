//! Native save-image state, separate from shared render resources. The fresh
//! bytes follow40EC50, sprite records follow444BC0/444180. Mutation coverage is
//! explicit: an unaudited accepted instruction prevents save-image admission.
use crate::native_save::{
    BODY_BYTES, CapturedNativeBody, LIVE_BASE, NativeReadRun, NativeSaveBody, READ_RUN_COUNT,
    SPRITE_BYTES, SPRITE_COUNT,
};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Serialize, Deserialize)]
pub struct NativeLiveState {
    #[serde(deserialize_with = "deserialize_body")]
    body: Vec<u8>,
    #[serde(deserialize_with = "deserialize_sprites")]
    sprites: Vec<Vec<u8>>,
    image_bindings: BTreeMap<u8, usize>,
    #[serde(default)]
    object_resources: Option<BTreeMap<u16, u8>>,
    #[serde(default)]
    held_images: Option<BTreeSet<u8>>,
    gaps: BTreeSet<String>,
    completed: BTreeSet<u8>,
    /// One-shot evidence produced by the source-specific projection hook for
    /// the instruction currently completing. It never erases an older gap.
    #[serde(default)]
    projection_receipts: BTreeSet<u8>,
    /// Native renderer968/972 and984/988 are deliberately outside the original
    /// slot body. Keep their source values separately; never infer from a slot.
    #[serde(default)]
    references: Option<Vec<NativeReference>>,
    #[serde(default)]
    movie: Option<NativeMovieShadow>,
    /// Constructor-proven empty primary queues, followed by acknowledged actor
    /// activity. None means an accepted queue producer has not been projected.
    #[serde(default)]
    known_queues: Option<BTreeSet<u16>>,
    #[serde(default)]
    primary_queues: BTreeSet<u16>,
    #[serde(default)]
    auxiliary_queues: BTreeSet<u16>,
    #[serde(default)]
    blur_activation: BTreeMap<u16, (Vec<u8>, bool)>,
    ///Only exact samples acknowledged by a zero-divisor shared track may
    ///seed a retained actor with nonfinite native bits.
    #[serde(default)]
    primary_ieee: BTreeMap<u16, BTreeMap<u8,u32>>,

}
#[derive(Clone, Debug, Serialize, Deserialize)]
struct NativeMovieShadow {
    #[serde(default = "backend_two")]
    backend: i16,
    record: Vec<u8>,
    playlist_count: usize,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
struct NativeReference {
    position: [f32; 2],
    scale: [f32; 2],
}
fn fresh_references() -> Vec<NativeReference> {
    vec![
        NativeReference {
            position: [0., 0.],
            scale: [100., 100.]
        };
        150
    ]
}
impl std::fmt::Debug for NativeLiveState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NativeLiveState")
            .field("body_bytes", &self.body.len())
            .field("sprites", &self.sprites.len())
            .field("gaps", &self.gaps)
            .finish_non_exhaustive()
    }
}
fn deserialize_body<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> std::result::Result<Vec<u8>, D::Error> {
    let bytes = Vec::<u8>::deserialize(d)?;
    if bytes.len() != BODY_BYTES {
        return Err(serde::de::Error::custom("native live body extent"));
    }
    Ok(bytes)
}
fn deserialize_sprites<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> std::result::Result<Vec<Vec<u8>>, D::Error> {
    let bytes = Vec::<Vec<u8>>::deserialize(d)?;
    if bytes.len() != SPRITE_COUNT || bytes.iter().any(|s| s.len() != SPRITE_BYTES) {
        return Err(serde::de::Error::custom("native live sprite extent"));
    }
    Ok(bytes)
}
fn backend_two() -> i16 { 2 }
fn movie_record_defaults(backend: i16) -> Vec<u8> {
    let mut record = vec![0; 1668];
    if backend == 1 {
        //4573D0→456F70→456EB0 initializes these six DWORDs. The
        //seventh copied DWORD is outside the consumed controls:457F30
        //never restores it and the backend1 methods do not read it.
        //Normalize only that opaque padding in a newly allocated record;
        //an imported slot keeps its exact four bytes, not original heap junk.
        record[1640..1644].copy_from_slice(&(-1i32).to_le_bytes());
        record[1644..1648].copy_from_slice(&1i32.to_le_bytes());
    }
    record
}
impl NativeLiveState {
    /// Native40EC50 zeroes the entire live body. These are actual initial VM
    /// bytes, never a serialization fallback for a partly known logical State.
    pub(crate) fn fresh() -> Self {
        Self {
            body: vec![0; BODY_BYTES],
            sprites: (0..SPRITE_COUNT).map(|_| fresh_sprite()).collect(),
            image_bindings: BTreeMap::new(),
            object_resources: Some(BTreeMap::new()),
            held_images: Some(BTreeSet::new()),
            gaps: BTreeSet::new(),
            completed: BTreeSet::new(),
            projection_receipts: BTreeSet::new(),
            references: Some(fresh_references()),
            movie: None,
            known_queues: Some(BTreeSet::new()),
            primary_queues: BTreeSet::new(),
            auxiliary_queues: BTreeSet::new(),
            blur_activation: BTreeMap::new(),
            primary_ieee: BTreeMap::new(),

        }
    }
    pub(crate) fn unavailable() -> Self {
        let mut state = Self::fresh();
        state.gap("legacy logical snapshot has no native live image");
        state
    }
    /// Import an authoritative original image into a *candidate* VM state.
    /// Publishing its engine resources is a separate required restore receipt.
    /// Unknown original bytes are retained, not inferred from logical State.
    pub(crate) fn from_original(body: &NativeSaveBody) -> Result<Self> {
        let mut state = Self {
            body: body.bytes().to_vec(),
            sprites: (0..SPRITE_COUNT)
                .map(|n| body.sprite(n).map(|s| s.bytes().to_vec()))
                .collect::<Result<_>>()?,
            image_bindings: BTreeMap::new(),
            object_resources: Some(BTreeMap::new()),
            held_images: Some(BTreeSet::new()),
            gaps: BTreeSet::new(),
            completed: BTreeSet::new(),
            projection_receipts: BTreeSet::new(),
            references: None,
            movie: None,
            known_queues: None,
            primary_queues: BTreeSet::new(),
            auxiliary_queues: BTreeSet::new(),
            blur_activation: BTreeMap::new(),
            primary_ieee: BTreeMap::new(),

        };
        for slot in 0..37u8 {
            if !body.image_name_bytes(usize::from(slot))?.is_empty() {
                state
                    .held_images
                    .as_mut()
                    .expect("authoritative restore ownership")
                    .insert(slot);
                if let Some(index) = sprite_for_slot(slot) {
                    state.image_bindings.insert(slot, index);
                    state
                        .object_resources
                        .as_mut()
                        .expect("restore ownership")
                        .insert(index as u16, slot);
                }
            }
        }
        Ok(state)
    }
    ///425640 copies only mapped nonempty resource records into renderers.
    /// Other actual renderers survive425DC0 (with its explicit clear effects).
    pub(crate) fn restore_renderer_scope(
        &mut self,
        previous: &Self,
        restored: &BTreeSet<u16>,
        reloaded: &BTreeSet<u8>,
        preserve: bool,
    ) -> Result<()> {
        let previous_aliases = previous
            .object_resources
            .as_ref()
            .context("Circus previous renderer ownership missing")?;
        let aliases = self
            .object_resources
            .as_mut()
            .context("Circus restored renderer ownership missing")?;
        for object in 0..150u16 {
            if restored.contains(&object) {
                continue;
            }
            self.sprites[usize::from(object)] = previous.sprites[usize::from(object)].clone();
            let sprite = &mut self.sprites[usize::from(object)];
            put_object_word(sprite, 908, 0);
            if !preserve {
                put_object_word(sprite, 740, 0);
                put_object_word(sprite, 904, if object == 0 { 0 } else { 50 });
            }
            if let Some(slot) = previous_aliases.get(&object) {
                let recursive = reloaded
                    .iter()
                    .any(|loaded| (4..=8).contains(loaded) && *slot == *loaded + 5);
                if reloaded.contains(slot) || recursive || (!preserve && !(37..=44).contains(slot))
                {
                    for offset in [64, 68, 184, 188] {
                        put_object_word(sprite, offset, 0);
                    }
                    aliases.remove(&object);
                } else {
                    aliases.insert(object, *slot);
                }
            }
        }
        Ok(())
    }
    pub(crate) fn restored_queues_acknowledged(&mut self) {
        self.known_queues = Some(
            self.primary_queues
                .union(&self.auxiliary_queues)
                .copied()
                .collect(),
        );
    }
    pub(crate) fn restoration_receipt(&mut self, opcode: u8) {
        self.projection_receipts.insert(opcode);
    }
    pub(crate) fn preserve_ephemeral_from(&mut self, previous: &Self) {
        if let (Some(held), Some(previous)) = (&mut self.held_images, &previous.held_images) {
            held.extend(
                previous
                    .iter()
                    .copied()
                    .filter(|slot| (37..=44).contains(slot)),
            );
        }
        self.references = previous.references.clone();
        self.primary_ieee=previous.primary_ieee.clone();
        self.movie = previous.movie.clone();
        if let Some(movie) = self.movie.as_mut() {
            if movie.backend == 1 {
                let padding = self.body[266340 - LIVE_BASE + 1664..266340 - LIVE_BASE + 1668].to_vec();
                movie.record = movie_record_defaults(1);
                movie.record[1664..1668].copy_from_slice(&padding);
            } else { movie.record[..1644].fill(0); }
            movie.playlist_count = 0;
        }
    }
    ///457D90/4586E0 create a complete zeroed1668-byte backend2 record. Keep
    /// it in player state;457F10 copies it into liveBuf only at41DD10.
    pub(crate) fn movie_backend_selected(&mut self, backend: Option<i16>) -> Result<()> {
        self.movie = match backend {
            Some(backend @ (1 | 2)) => Some(NativeMovieShadow {
                backend, record: movie_record_defaults(backend), playlist_count: 0,
            }),
            None => None,
            _ => anyhow::bail!("Circus native Movie backend index"),
        };
        //This obsolete gap incorrectly treated all28 controls as unknown.
        //Known24 bytes are now initialized by456EB0; the last4 are opaque
        //padding, with canonical-new/import-preserving provenance above.
        self.gaps.remove("backend1 Movie ctor leaves copied28-byte save tail uninitialized");
        self.projection_receipts.insert(0xab);
        Ok(())
    }
    pub(crate) fn comment_record_for_backend(&mut self, backend: i16, record: &[u8]) -> Result<()> {
        ensure!(matches!(backend, 1 | 2), "Circus live comment backend record owner");
        if self.movie.as_ref().is_none_or(|movie| movie.backend != backend) {
            self.movie = Some(NativeMovieShadow { backend, record: movie_record_defaults(backend), playlist_count: 0 });
        }
        self.comment_record(record)
    }
    pub(crate) fn comment_record(&mut self, record: &[u8]) -> Result<()> {
        ensure!(record.len() == 1668, "Circus native comment record extent");
        let movie = self
            .movie
            .as_mut()
            .context("Circus native comment backend shadow unavailable")?;
        movie.record.copy_from_slice(record);
        if movie.backend==1 {
            movie.playlist_count=record[..1640].chunks_exact(82).take_while(|row|row[0]!=0).count();
        }
        self.gaps.remove("backend1 Movie ctor leaves copied28-byte save tail uninitialized");
        self.projection_receipts.insert(0xab);
        Ok(())
    }
    pub(crate) fn comment_control_completed(&mut self) {
        self.projection_receipts.insert(0xab);
    }
    pub(crate) fn movie_playlist(&mut self, name: &str) -> Result<()> {
        let (bytes, _, error) = encoding_rs::SHIFT_JIS.encode(name);
        ensure!(
            !error && bytes.len() < 82 && !bytes.contains(&0),
            "Circus native Movie playlist string"
        );
        if let Some(movie) = self.movie.as_mut() {
            if movie.playlist_count < if movie.backend == 1 { 20 } else { 200 } {
                let at = 82 * movie.playlist_count;
                for (n, value) in bytes.iter().copied().chain(std::iter::once(0)).enumerate() {
                    if let Some(out) = movie.record.get_mut(at + n) {
                        *out = value;
                    }
                }
                movie.playlist_count += 1;
            }
        }
        self.projection_receipts.insert(0x6d);
        Ok(())
    }
    pub(crate) fn movie_stopped(&mut self) {
        if let Some(movie) = self.movie.as_mut() {
            if movie.backend == 1 {
                let padding = movie.record[1664..1668].to_vec();
                movie.record = movie_record_defaults(1);
                movie.record[1664..1668].copy_from_slice(&padding);
            } else { movie.record[..1644].fill(0); }
            movie.playlist_count = 0;
        }
        self.projection_receipts.insert(0xab);
    }
    pub(crate) fn stage_reference(&self, object: u16) -> Result<([f32; 2], [f32; 2])> {
        ensure!(object < 150, "Circus native reference object index");
        let r = self
            .references
            .as_ref()
            .and_then(|refs| refs.get(usize::from(object)))
            .context("Circus original renderer reference metadata unavailable")?;
        Ok((r.position, r.scale))
    }
    pub(crate) fn stage_reference_commit(
        &mut self,
        object: u16,
        position: Option<[f32; 2]>,
        scale: Option<[f32; 2]>,
    ) -> Result<()> {
        ensure!(object < 150, "Circus native stage reference object");
        ensure!(
            position
                .iter()
                .chain(scale.iter())
                .flatten()
                .all(|v| v.is_finite()),
            "Circus native stage reference nonfinite scalar"
        );
        let index = usize::from(object);
        let raw = 121248 - LIVE_BASE + index * 960;
        if let Some(position) = position {
            for (at, value) in [
                (8, position[0]),
                (12, position[1]),
                (56, position[0]),
                (60, position[1]),
            ] {
                put_object_float(&mut self.sprites[index], at, value);
            }
            for (n, value) in position.into_iter().enumerate() {
                self.body[raw + n * 4..raw + n * 4 + 4].copy_from_slice(&value.to_le_bytes());
            }
        }
        if let Some(scale) = scale {
            for (at, value) in [(128, scale[0]), (132, scale[1])] {
                put_object_float(&mut self.sprites[index], at, value);
                self.body[raw + at - 8..raw + at - 4].copy_from_slice(&value.to_le_bytes());
            }
        }
        Ok(())
    }
    pub(crate) fn stage_reference_cache(
        &mut self,
        object: u16,
        position: Option<[f32; 2]>,
        scale: Option<[f32; 2]>,
    ) -> Result<()> {
        ensure!(object < 150, "Circus native stage reference object");
        ensure!(
            position
                .iter()
                .chain(scale.iter())
                .flatten()
                .all(|v| v.is_finite()),
            "Circus native stage reference nonfinite scalar"
        );
        let index = usize::from(object);
        let raw = 121248 - LIVE_BASE + index * 960;
        if let Some(position) = position {
            for (at, value) in [(56, position[0]), (60, position[1])] {
                put_object_float(&mut self.sprites[index], at, value);
            }
            for (n, value) in position.into_iter().enumerate() {
                self.body[raw + n * 4..raw + n * 4 + 4].copy_from_slice(&value.to_le_bytes());
            }
        }
        if let Some(scale) = scale {
            put_object_float(&mut self.sprites[index], 136, scale[0]);
            for (n, value) in scale.into_iter().enumerate() {
                self.body[raw + 120 + n * 4..raw + 124 + n * 4]
                    .copy_from_slice(&value.to_le_bytes());
            }
        }
        Ok(())
    }
    fn remember_position(&mut self, object: usize) {
        if let Some(refs) = self.references.as_mut() {
            refs[object].position = [
                object_float(&self.sprites[object], 8),
                object_float(&self.sprites[object], 12),
            ];
        }
    }
    fn remember_scale(&mut self, object: usize) {
        if let Some(refs) = self.references.as_mut() {
            refs[object].scale = [
                object_float(&self.sprites[object], 128),
                object_float(&self.sprites[object], 132),
            ];
        }
    }
    pub fn gaps(&self) -> &BTreeSet<String> {
        &self.gaps
    }
    pub fn completed_opcodes(&self) -> &BTreeSet<u8> {
        &self.completed
    }
    pub(crate) fn gap(&mut self, reason: impl Into<String>) {
        self.gaps.insert(reason.into());
    }
    /// Decode/record-only continuation: no executed opcode is inferred.
    pub(crate) fn advance_pc(&mut self, scene: &str, pc: u32) -> Result<()> {
        self.validate()?;
        self.cstring(19336, 128, scene)?;
        self.write32(22132, pc);
        Ok(())
    }
    ///433E50 clears50 records immediately before normal4284B0 invocation.
    pub(crate) fn clear_macro_records(&mut self) {
        self.body[269852 - LIVE_BASE..375652 - LIVE_BASE].fill(0);
    }
    pub(crate) fn macro_record(
        &mut self,
        group: u8,
        index: u8,
        opcode: u8,
        args: [i16; 8],
        strings: &[String; 8],
    ) -> Result<()> {
        ensure!(group < 10 && index < 10, "Circus native macro record index");
        if group >= 5 {
            return Ok(());
        }
        let base = 269852 - LIVE_BASE + 2116 * (usize::from(group) * 10 + usize::from(index));
        let mut encoded = Vec::with_capacity(8);
        for text in strings {
            let (bytes, _, error) = encoding_rs::SHIFT_JIS.encode(text);
            ensure!(
                !error && !bytes.contains(&0),
                "Circus native macro string encoding"
            );
            encoded.push(bytes.into_owned());
        }
        self.body[base..base + 4].copy_from_slice(&u32::from(opcode).to_le_bytes());
        for n in 0..8 {
            let arg = base + 2084 + n * 4;
            self.body[arg..arg + 4].copy_from_slice(&i32::from(args[n]).to_le_bytes());
            //strlen>=260 skips this copy, retaining prior string bytes.
            if encoded[n].len() < 260 {
                let at = base + 4 + n * 260;
                self.body[at..at + encoded[n].len()].copy_from_slice(&encoded[n]);
                self.body[at + encoded[n].len()] = 0;
            }
        }
        Ok(())
    }
    fn validate(&self) -> Result<()> {
        ensure!(self.primary_ieee.len()<=150&&self.primary_ieee.iter().all(|(object,channels)|
            *object<150&&channels.len()<=17&&channels.iter().all(|(channel,bits)|*channel<17&&!f32::from_bits(*bits).is_finite())),
            "Circus native IEEE provenance extent");
        ensure!(
            self.body.len() == BODY_BYTES
                && self.sprites.len() == SPRITE_COUNT
                && self.sprites.iter().all(|s| s.len() == SPRITE_BYTES),
            "Circus native live image extent"
        );
        ensure!(
            self.movie
                .as_ref()
                .is_none_or(|movie| movie.record.len() == 1668 && movie.playlist_count <= 200),
            "Circus native Movie save image extent"
        );
        Ok(())
    }
    fn write32(&mut self, absolute: usize, value: u32) {
        let at = absolute - LIVE_BASE;
        self.body[at..at + 4].copy_from_slice(&value.to_le_bytes());
    }
    fn read32(&self, absolute: usize) -> u32 {
        let at = absolute - LIVE_BASE;
        u32::from_le_bytes(self.body[at..at + 4].try_into().unwrap())
    }
    fn read16(&self, absolute: usize) -> u16 {
        let at = absolute - LIVE_BASE;
        u16::from_le_bytes(self.body[at..at + 2].try_into().unwrap())
    }
    fn cstring(&mut self, absolute: usize, capacity: usize, text: &str) -> Result<()> {
        let (bytes, _, errors) = encoding_rs::SHIFT_JIS.encode(text);
        ensure!(
            !errors && bytes.len() < capacity && !bytes.contains(&0),
            "Circus native live string extent/encoding"
        );
        let at = absolute - LIVE_BASE;
        self.body[at..at + bytes.len()].copy_from_slice(&bytes);
        self.body[at + bytes.len()] = 0;
        Ok(())
    }
    pub(crate) fn byte_assignment(&mut self, bank: i16, index: i16, value: u8) {
        if (0..=10).contains(&bank) && (0..100).contains(&index) {
            self.body[118892 - LIVE_BASE + bank as usize * 100 + index as usize] = value;
        }
    }
    pub(crate) fn system_assignment(&mut self, index: i16, value: i32) {
        //40D240's body-backed scalar setters. Preference-prefix and
        //window state remain distinct;183 is the word overlapping voice code.
        if let Some(at) = match index {
            183 => Some(66584*4),
            233 => Some(115880),
            241 => Some(118888),
            _ => None,
        } {
            self.write32(at, value as u32);
        }
    }
    ///40CC50 reads live native words, including writes by non-assignment
    ///producers. Do not substitute a stale logical system-variable cache.
    pub(crate) fn runtime_system_value(&self,selector:i16)->Option<i32> {
        let word=match selector{183=>66584,233=>28970,234=>66329,241=>29722,_=>return None};
        Some(self.read32(word*4) as i32)
    }
    ///4284B0 case158 stores the native scene marker atDWORD66508.
    pub(crate) fn scene_marker(&mut self, value: i16) {
        self.write32(66508 * 4, value as i32 as u32);
    }
    ///402640 pops only the trailing current-scene native read runs. It retains
    /// popped record bytes and restores the original byte/random history cursors.
    pub(crate) fn discard_scene_read_history(&mut self, scene: i32) -> Result<()> {
        self.validate()?;
        let mut body = NativeSaveBody::from_exact(&self.body)?;
        let mut index = body.read_run_index() as usize;
        ensure!(index < READ_RUN_COUNT, "Circus native read history index");
        while index != 0 {
            let run = body.read_run(index)?;
            if i32::from(run.scene_index as i16) != scene {
                break;
            }
            self.write32(27965 * 4, u32::from(run.source_word_55930));
            self.write32(27967 * 4, u32::from(run.source_word_55934));
            index -= 1;
        }
        let run = body.read_run(index)?;
        if i32::from(run.scene_index as i16) == scene {
            // Only the first WORD becomes the -1 sentinel, preserving all
            // following native metadata bytes of this retained record.
            let at = 106240 - LIVE_BASE + index * 14;
            self.body[at..at + 2].copy_from_slice(&u16::MAX.to_le_bytes());
            self.write32(27965 * 4, 0);
            self.write32(27967 * 4, 0);
        }
        body.set_read_run_index(index)?;
        self.write32(106236, index as u32);
        self.write32(27964 * 4, self.read32(27965 * 4));
        self.write32(27966 * 4, self.read32(27967 * 4));
        self.write32(27962 * 4, 0);
        self.write32(27963 * 4, 0);
        self.write32(27968 * 4, self.read32(27969 * 4));
        Ok(())
    }
    pub(crate) fn reset_script(&mut self, clear_banks: bool, names: &[String; 2]) -> Result<()> {
        self.validate()?;
        if clear_banks {
            self.body[118892 - LIVE_BASE..121092 - LIVE_BASE].fill(0);
        }
        self.cstring(265268, 20, &names[1])?;
        self.cstring(265288, 20, &names[0])?;
        self.body[106240 - LIVE_BASE..106242 - LIVE_BASE].copy_from_slice(&u16::MAX.to_le_bytes());
        for at in [
            106236, 111856, 111860, 111864, 111868, 111872, 115884, 115880,
        ] {
            self.write32(at, 0);
        }
        Ok(())
    }
    ///4284B0 case10: the normal nonzero selector changes only prefix
    ///auto/skip preferences or CWnd interaction state, never this live body.
    ///Zero-selector message refresh and positive history dispatch are separate
    ///writers and must not inherit this receipt.
    pub(crate) fn read_check_state(&mut self, unread: bool) {
        self.write32(66509 * 4, u32::from(unread));
        self.projection_receipts.insert(0x03);
    }
    pub(crate) fn dialogue_marker(&mut self, opcode: u8, enabled: bool) -> Result<()> {
        ensure!(matches!(opcode, 0x06 | 0x09), "Circus dialogue marker producer");
        self.write32(66515 * 4, u32::from(enabled));
        self.projection_receipts.insert(opcode);
        Ok(())
    }
    pub(crate) fn dialogue_refresh_buffers(&self) -> Result<(String, String)> {
        let body = NativeSaveBody::from_exact(&self.body)?;
        Ok((body.native_string(265812, LIVE_BASE + BODY_BYTES - 265812)?,
            body.native_string(266072, LIVE_BASE + BODY_BYTES - 266072)?))
    }
    ///41FB00 consumes one of the two-hundred retained16-byte action names.
    ///A missing named label still consumes its entry, leaving the decoder PC.
    pub(crate) fn dialogue_replay_action(&mut self) -> Result<String> {
        let cursor = self.read32(27964 * 4) as usize;
        ensure!(cursor < 200, "Circus dialogue action history exhausted");
        let body = NativeSaveBody::from_exact(&self.body)?;
        let action = body.native_string(111880 + cursor * 16, 16)?;
        self.write32(27964 * 4, cursor as u32 + 1);
        self.write32(66515 * 4, 0);
        self.projection_receipts.insert(0x0a);
        Ok(action)
    }
    pub(crate) fn dialogue_label_replay(&mut self, replay_target: i32) -> Result<Option<u16>> {
        let body = NativeSaveBody::from_exact(&self.body)?;
        let index = body.read_run_index() as usize;
        let row = body.read_run(index)?;
        self.write32(66515 * 4, 0);
        let mut target = None;
        if u32::from(row.last_label) == self.replay_label() && index + 1 < READ_RUN_COUNT {
            let next = body.read_run(index + 1)?.first_label;
            if u32::from(next) > self.replay_label() && replay_target >= i32::from(next) && next > 1 {
                self.write32(111848, u32::from(next));
                target = Some(next);
            }
        }
        self.projection_receipts.insert(0x0a);
        Ok(target)
    }
    ///433E50 successful forward fromprocess6, after the actual text/audio
    ///consumer ACK. Processed body/name text buffers deliberately survive.
    pub(crate) fn dialogue_forward_completed(&mut self) {
        self.body[265392-LIVE_BASE]=0;
        for index in 0..4 {if self.read32(22112+index*4)==0{self.body[21984+index*32-LIVE_BASE]=0;}}
    }
    pub(crate) fn dialogue_refresh_completed(&mut self, message_wait: bool) {
        if message_wait { self.write32(66510 * 4, 1); }
        self.projection_receipts.insert(0x0a);
    }
    pub(crate) fn dialogue_nonzero_completed(&mut self, selector: i16, dispatch: i32, forced: bool, label_replay: bool) {
        if selector != 0 && dispatch == 0 && !forced && !label_replay {
            self.projection_receipts.insert(0x0a);
        }
    }
    pub(crate) fn replay_label(&self) -> u32 { self.read32(111848) }
    pub(crate) fn take_replay_random(&mut self) -> Result<i32> {
        let cursor = self.read32(111864) as usize;
        ensure!(cursor < 200, "Circus replay random history exhausted");
        let value = self.read32(115080 + 4 * cursor) as i32;
        self.write32(111864, cursor as u32 + 1);
        Ok(value)
    }
    pub(crate) fn random_result(&mut self, value: i32) {
        let count = self.read32(111864) as usize;
        if count < 200 {
            self.write32(115080 + 4 * count, value as u32);
            self.write32(111864, count as u32 + 1);
        }
    }
    ///402A10 records changed byte-bank entries in ascending index order. Even
    /// after the1000-record history is full, the comparison bank is updated.
    pub(crate) fn record_scene_bank_changes(&mut self) -> Result<()> {
        let mut count = self.read32(115884) as usize;
        ensure!(count <= 1000, "Circus native byte-change history extent");
        for index in 0..1100 {
            let current = self.body[118892 - LIVE_BASE + index];
            let previous = self.body[119992 - LIVE_BASE + index];
            if current == previous {
                continue;
            }
            if count < 1000 {
                let at = 115888 - LIVE_BASE + count * 2;
                self.body[at..at + 2].copy_from_slice(&(index as u16).to_le_bytes());
                self.body[117888 - LIVE_BASE + count] = current;
                count += 1;
            }
            self.body[119992 - LIVE_BASE + index] = current;
        }
        self.write32(115884, count as u32);
        Ok(())
    }
    /// Opcode63 appends only when the original Shift-JIS byte extent remains
    /// below128, retaining the previous string when a suffix does not fit.
    pub(crate) fn append_version(&mut self, suffix: &str) -> Result<bool> {
        let at = 265848 - LIVE_BASE;
        let length = self.body[at..at + 128]
            .iter()
            .position(|b| *b == 0)
            .ok_or_else(|| anyhow::anyhow!("Circus native version unterminated"))?;
        let (bytes, _, errors) = encoding_rs::SHIFT_JIS.encode(suffix);
        ensure!(
            !errors && !bytes.contains(&0),
            "Circus native version encoding"
        );
        if length + bytes.len() >= 128 {
            return Ok(false);
        }
        self.body[at + length..at + length + bytes.len()].copy_from_slice(&bytes);
        self.body[at + length + bytes.len()] = 0;
        Ok(true)
    }
    pub(crate) fn music(&mut self, id: i16, name: &str) -> Result<()> {
        self.cstring(21944, 32, name)?;
        self.write32(21976, id as u32);
        Ok(())
    }
    pub(crate) fn inline_music_declared(&mut self,id:i32) {
        //41FBE0's @mu constructs a Main-heap timed event and writes5494
        //immediately.21944 remains the previously played name until its event.
        self.write32(21976,id as u32);
    }
    pub(crate) fn speaker_preprocessed(&mut self,opcode:u8)->Result<()> {
        ensure!(matches!(opcode,0x5e|0x6a),"Circus speaker declaration receipt");
        //4284B0 cases94/106 build @s+text in Main11492, then41FBE0
        //edits that buffer and Main5985..6117 event fields. Its only Buf
        //write is @mu's5494, projected separately above. In particular the
        //saved265812 speaker and265392 voice source are not written here.
        self.projection_receipts.insert(opcode);Ok(())
    }
    pub(crate) fn music_volume_backup(&mut self, value: i32) {
        self.write32(66514 * 4, value as u32);
    }
    pub(crate) fn media_completed(&mut self, opcode: u8, selector: i16) {
        //71's positive selector only declares a dispatcher-local pending
        //SE.43C390 writes21984..22111 later, after the real decoder/publish
        //ACK (sound_loaded); -1/-2 are projected by their explicit helpers.
        if opcode == 0x6f || opcode == 0x71 || opcode == 0x05 && selector != 254 {
            self.projection_receipts.insert(opcode);
        }
    }
    pub(crate) fn audio_transport_completed(&mut self) {
        //4284B0 case169: selectors0/1/2 call43B820;10/11 call
        //43AED0/43AF10 and only change audio-heap/DirectSound transport.
        //Selector0 also assigns99/187 via40D240 to Buf297158, outside
        //this19328..375652 body.99 only records dispatcher-local v512;
        //its43B2C0 tail updates the audio object's retained loop offset.
        //All other selectors are original switch no-ops. None writes a
        //saved voice/name/loop field; their actual host ACK precedes this.
        self.projection_receipts.insert(0xa9);
    }
    ///43C390 copies the *original resolved name* after successful decoding on
    /// actual physical SE channels2..5;43AB90 saves only channel2's loop flag.
    pub(crate) fn sound_loaded(&mut self, channel: u32, name: &str, looping: bool) -> Result<()> {
        if (2..=5).contains(&channel) {
            self.cstring(21920 + channel as usize * 32, 32, name)?;
        }
        if channel == 2 {
            self.write32(22112, u32::from(looping));
        }
        Ok(())
    }
    ///Native67/6B use unchecked strcpy into the original contiguous Buf.
    ///Model in-bounds overlaps exactly, without importing unsafe memory writes.
    pub(crate) fn voice_code(&mut self, name: &str) -> Result<()> {
        self.cstring(
            266332,
            LIVE_BASE + BODY_BYTES - 266332,
            &name.to_ascii_uppercase(),
        )?;
        self.projection_receipts.insert(0x67);
        Ok(())
    }
    pub(crate) fn dialogue_buffers(
        &mut self,
        speaker: &str,
        processed_body: &str,
        cut_marker: Option<&str>,
    ) -> Result<()> {
        self.cstring(266072, LIVE_BASE + BODY_BYTES - 266072, processed_body)?;
        self.cstring(265812, LIVE_BASE + BODY_BYTES - 265812, speaker)?;
        self.cstring(265552, LIVE_BASE + BODY_BYTES - 265552, processed_body)?;
        if let Some(marker) = cut_marker.filter(|marker| !marker.is_empty()) {
            let (needle, _, error) = encoding_rs::SHIFT_JIS.encode(marker);
            ensure!(!error, "Circus dialogue truncation marker encoding");
            let (bytes, _, error) = encoding_rs::SHIFT_JIS.encode(processed_body);
            ensure!(!error, "Circus dialogue source encoding");
            if let Some(index) = bytes
                .windows(needle.len())
                .position(|window| window == needle.as_ref())
            {
                self.body[265552 - LIVE_BASE + index] = 0;
            }
        }
        self.projection_receipts.insert(0x6b);
        Ok(())
    }
    pub(crate) fn voice_source(&mut self, opcode: u8, name: &str) -> Result<()> {
        ensure!(
            matches!(opcode, 0x4c | 0x4d | 0x4e | 0x71),
            "Circus native voice-source opcode"
        );
        self.cstring(265392, 128, name)?;
        self.projection_receipts.insert(opcode);
        Ok(())
    }
    pub(crate) fn sound_stop_declared(&mut self, protected: bool) {
        if !protected {
            self.body[21984 - LIVE_BASE] = 0;
        }
        self.projection_receipts.insert(0x71);
    }
    pub(crate) fn sound_stopped(&mut self, channel: u32) {
        if (2..=5).contains(&channel) {
            self.body[21920 + 32 * channel as usize - LIVE_BASE] = 0;
        }
    }
    pub(crate) fn audio_publication_completed(&mut self) {
        self.projection_receipts.insert(0x05);
    }
    pub(crate) fn full_image_immediate(&mut self, position: [i32; 2], percent: i16) {
        let s = &mut self.sprites[2];
        for (at, value) in [
            (8, position[0] as f32),
            (12, position[1] as f32),
            (56, position[0] as f32),
            (60, position[1] as f32),
            (128, percent as f32),
            (132, percent as f32),
        ] {
            put_object_float(s, at, value);
        }
        self.projection_receipts.insert(0xac);
    }
    pub(crate) fn placement(&mut self, kind: i16, name: &str, position: [i16; 2]) -> Result<()> {
        ensure!((0..20).contains(&kind), "Circus native placement table");
        let count_at = 106156 + kind as usize * 4;
        let count = self.read32(count_at) as usize;
        ensure!(count <= 100, "Circus native placement capacity");
        let base = 26152 + kind as usize * 4000;
        let name = name.to_ascii_uppercase();
        let found = (0..count).find(|index| {
            let at = base + index * 40 - LIVE_BASE;
            let row = &self.body[at..at + 32];
            let end = row.iter().position(|b| *b == 0).unwrap_or(32);
            row[..end].eq_ignore_ascii_case(name.as_bytes())
        });
        let index = found.unwrap_or(count);
        ensure!(index < 100, "Circus native placement capacity");
        self.cstring(base + index * 40, 32, &name)?;
        self.write32(base + index * 40 + 32, position[0] as i32 as u32);
        self.write32(base + index * 40 + 36, position[1] as i32 as u32);
        if found.is_none() {
            self.write32(count_at, count as u32 + 1);
        }
        Ok(())
    }
    pub(crate) fn voice_timing(&mut self, mode: i16, name: &str) -> Result<()> {
        ensure!(
            mode == 255 || name.starts_with('-') || (name.is_ascii() && name.len() < 32),
            "Circus voice timing prefix extent/encoding"
        );
        self.write32(269848, 0);
        if mode == 255 || name.starts_with('-') {
            self.body[268008 - LIVE_BASE..269844 - LIVE_BASE].fill(0);
            self.write32(269844, 0);
            if mode == 255 {
                self.write32(269848, 1);
            }
            return Ok(());
        }
        let count = self.read32(269844) as usize;
        ensure!(count <= 50, "Circus voice timing table capacity");
        for index in 0..count {
            let at = 268012 + index * 36 - LIVE_BASE;
            let row = &self.body[at..at + 32];
            if row.len() >= name.len() && row[..name.len()].eq_ignore_ascii_case(name.as_bytes()) {
                return Ok(());
            }
        }
        ensure!(count < 50, "Circus voice timing table capacity");
        let gain = 100
            * if mode > 100 {
                i32::from(mode) - 256
            } else {
                i32::from(mode)
            };
        self.write32(268008 + count * 36, gain as u32);
        self.cstring(268012 + count * 36, 32, &name.to_ascii_uppercase())?;
        self.write32(269844, count as u32 + 1);
        Ok(())
    }
    pub(crate) fn read_label(
        &mut self,
        scene_index: i32,
        label: u16,
        metadata: u16,
        raw: u32,
        track: bool,
    ) -> Result<()> {
        self.validate()?;
        if track {
            let mut image = NativeSaveBody::from_exact(&self.body)?;
            let mut index = image.read_run_index() as usize;
            ensure!(index < READ_RUN_COUNT, "Circus native live read-run index");
            let old = image.read_run(index)?;
            let flags = self.read16(115884) | if self.read32(115880) != 0 { 0x8000 } else { 0 };
            let mut run = if old.scene_index == scene_index as u16
                && u32::from(old.last_label) + 1 == u32::from(label)
            {
                NativeReadRun {
                    last_label: label,
                    metadata_and_flags: flags,
                    ..old
                }
            } else {
                if index < 399 && old.scene_index != u16::MAX {
                    let duplicate = (0..index).rev().any(|n| {
                        image.read_run(n).is_ok_and(|p| {
                            p.scene_index == old.scene_index
                                && p.last_label == old.last_label
                                && p.first_label <= old.first_label
                        })
                    });
                    if !duplicate {
                        index += 1;
                        image.set_read_run_index(index)?;
                    }
                }
                NativeReadRun {
                    scene_index: scene_index as u16,
                    metadata_and_flags: flags,
                    first_label: label,
                    last_label: label,
                    source_word_55936: self.read16(111872),
                    source_word_55930: self.read16(111860),
                    source_word_55934: self.read16(111868),
                }
            };
            // The assignment above deliberately retains all metadata when an
            // existing contiguous run is extended, matching4036D0.
            run.metadata_and_flags = flags;
            image.set_read_run(index, run)?;
            self.body.copy_from_slice(image.bytes());
            self.write32(111848, u32::from(label));
            if (raw as i32) < 0 {
                self.write32(111872, self.read32(111872).wrapping_add(1));
            }
        }
        if self.read32(111844) == 0 {
            self.write32(111844, u32::from(label));
        }
        self.write32(22136, u32::from(metadata));
        self.write32(22140, u32::from(label));
        Ok(())
    }
    pub(crate) fn image_name(&mut self, slot: u8, text: &str) -> Result<()> {
        ensure!(slot < 51, "Circus native image-name slot");
        //422CF0 uppercases its local filename before loading and copying it
        // into the original live slot table. This is format state, not an
        // ambient filesystem path or an engine texture identity.
        self.cstring(
            19496 + usize::from(slot) * 48,
            48,
            &text.to_ascii_uppercase(),
        )
    }
    ///422CF0 releases the previous texture before loading and copying a new
    /// name. This differs from431F20's ownership transfer, which retains it.
    pub(crate) fn load_image(&mut self, slot: u8, text: &str) -> Result<()> {
        self.unload_image(slot)?;
        self.image_name(slot, text)?;
        self.held_images
            .as_mut()
            .context("Circus native image ownership unavailable")?
            .insert(slot);
        Ok(())
    }
    pub(crate) fn shader_mode(&mut self, mode: u8) -> Result<()> {
        ensure!(mode < 4, "Circus native shader mode");
        for s in &mut self.sprites[..60] {
            put_object_word(s, 960, u32::from(mode));
        }
        self.projection_receipts.insert(0xb3);
        Ok(())
    }
    pub(crate) fn render_alignment(&mut self,object:u16,mode:u32)->Result<()> {
        ensure!(object<150,"Circus renderer alignment index");
        put_object_word(&mut self.sprites[usize::from(object)],964,mode);Ok(())
    }
    pub(crate) fn background_fit(&mut self) {
        self.write32(66517 * 4, 1);
        self.projection_receipts.insert(0xb4);
    }
    /// Verified stage declaration words; this does not acknowledge the native
    /// queued animation finalization that can accompany9A..9D.
    pub(crate) fn stage_pending(&mut self, pending: bool, reset_zoom: bool) {
        self.write32(66513 * 4, u32::from(pending));
        if reset_zoom {
            self.write32(66512 * 4, 0);
        }
    }
    pub(crate) fn stage_zoom_idle(&self) -> Result<bool> {
        self.validate()?;
        Ok([0, 10, 20, 30, 40, 50].into_iter().all(|n| {
            object_float(&self.sprites[n], 736) == 100.
                && object_float(&self.sprites[n], 36) == 1.
                && object_word(&self.sprites[n], 740) == 0
        }))
    }
    pub(crate) fn native_primary_queues_idle(&self) -> Result<bool> {
        self.validate()?;
        Ok(self.known_queues.as_ref().is_some_and(BTreeSet::is_empty)
            && self.sprites.iter().all(|sprite| {
                (632..=672)
                    .step_by(4)
                    .all(|offset| object_word(sprite, offset) == 0)
            }))
    }
    pub(crate) fn stage_auxiliary(
        &mut self,
        object: u16,
        scale: [f32; 2],
        pivot: [f32; 2],
        enabled: bool,
        declared: bool,
    ) -> Result<()> {
        ensure!(
            object < 150 && scale.iter().chain(pivot.iter()).all(|v| v.is_finite()),
            "Circus native auxiliary transform"
        );
        let s = &mut self.sprites[usize::from(object)];
        for (at, value) in [
            (36, scale[0]),
            (40, scale[1]),
            (44, pivot[0]),
            (48, pivot[1]),
        ] {
            put_object_float(s, at, value);
        }
        put_object_word(s, 52, u32::from(declared));
        put_object_word(s, 740, u32::from(enabled));
        Ok(())
    }
    pub(crate) fn stage_auxiliary_sample(&mut self,object:u16,scale:[f32;2],pivot:[f32;2],enabled:bool,declared:bool,status:&crysirror_scene::layer::queued::SequenceStatus)->Result<()> {
        let values=[scale[0],scale[1],pivot[0],pivot[1]];
        ensure!(object<150&&status.values.len()==4,"Circus auxiliary sample identity");
        for (channel,value) in values.into_iter().enumerate() {
            ensure!(value.is_finite()||(status.allows_nonfinite_channel(channel as u8)
                &&status.values[channel].to_bits()==value.to_bits()),"Circus auxiliary nonfinite sample lacks zero-divisor provenance");
        }
        let s=&mut self.sprites[usize::from(object)];
        for (at,value) in [(36,scale[0]),(40,scale[1]),(44,pivot[0]),(48,pivot[1])]{put_object_float(s,at,value);}
        put_object_word(s,52,u32::from(declared));put_object_word(s,740,u32::from(enabled));
        Ok(())
    }
    pub(crate) fn stage_sequence_status(
        &mut self,
        object: u16,
        status: &crysirror_scene::layer::queued::SequenceStatus,
    ) -> Result<()> {
        use crysirror_scene::layer::queued::{Active, SequenceTrack};
        ensure!(object < 150, "Circus stage sequence object");
        if status.lanes.values().any(|lane| !lane.records.is_empty()) {
            self.auxiliary_queues.insert(object);
        } else {
            self.auxiliary_queues.remove(&object);
        }
        if let Some(active) = self.known_queues.as_mut() {
            *active = self
                .primary_queues
                .union(&self.auxiliary_queues)
                .copied()
                .collect();
        }
        let s = &mut self.sprites[usize::from(object)];
        let Some(lane) = status.lanes.get(&0) else {
            put_object_word(s, 652, 0);
            return Ok(());
        };
        put_object_word(s, 652, u32::from(lane.active.is_some()));
        put_object_word(s, 696, u32::from(lane.cursor));
        if let (
            Some(record),
            Some(Active::Linear {
                velocity,
                multiplier,
            }),
        ) = (&lane.last_record, &lane.last_active)
        {
            let SequenceTrack::Linear { target, .. } = &record.track else {
                anyhow::bail!("Circus stage queue track kind")
            };
            ensure!(
                target.len() == 4 && velocity.len() == 4,
                "Circus stage queue field arity"
            );
            for (at, value) in [
                (396, target[0]),
                (408, target[2]),
                (412, target[3]),
                (424, velocity[0]),
                (416, velocity[2]),
                (420, velocity[3]),
                (428, *multiplier),
            ] {
                put_object_float(s, at, value);
            }
        }
        Ok(())
    }
    /// Audited18 selectors whose only non-PC writes are settings before19328,
    /// retained375652 mirror bytes, CWnd/GUI state or platform state. Selectors6
    /// and18 require their source402640/special-resource hooks before the
    ///completed dispatch reaches this receipt; neither hook is replaced here.
    pub(crate) fn syscall_completed(&mut self, selector: i16) {
        //22/23/25 now complete actual auth/network/disk ownership before
        //this receipt.426300's temporary375652 mirror is restored; only the
        //settings prefix/slotfiles/account globals survive outside livebody.
        if matches!(selector,1..=29|99) || !matches!(selector,0..=29|99) {
            self.projection_receipts.insert(0x18);
        }
    }
    ///40DB20's separate100-row auxiliary-pivot table, before placement kind0.
    pub(crate) fn character_auxiliary(&self,name:&str)->Result<[i32;2]> {
        let body=NativeSaveBody::from_exact(&self.body)?;
        let count=body.native_word(106152)? as i32;
        if count<=0{return Ok([0,0])}
        ensure!(count<=100,"Circus character auxiliary table extent");
        for index in 0..count as usize {
            let at=22152+40*index;
            let prefix=body.native_string(at,32)?;
            if name.contains(&prefix){return Ok([body.native_word(at+32)? as i32,body.native_word(at+36)? as i32])}
        }
        Ok([0,0])
    }
    pub(crate) fn character_preview_completed(&mut self) {
        self.projection_receipts.insert(0x1e);
    }
    pub(crate) fn protagonist_difference(&mut self, different: bool) {
        self.write32(66516 * 4, u32::from(different));
        self.projection_receipts.insert(0x1e);
    }
    pub(crate) fn ignored_system_dispatch(&mut self) {
        self.projection_receipts.insert(0x1e);
    }
    pub(crate) fn system_function_completed(&mut self, selector: i16, parameter: i16) {
        if matches!(
            selector,
            1 | 2 | 6 | 7 | 9 | 18 | 19 | 20 | 98 | 99 | 105 | 200 | 201 | 202 | 204 | 206 | 211
        ) || selector == 51 && parameter != 1
        {
            self.projection_receipts.insert(0x1e);
        }
    }
    pub(crate) fn ignored_framebuffer_effect(&mut self, kind: i16) {
        if matches!(kind, 7 | 10 | 11 | 13 | 14 | 23 | 24 | 25 | 30 | 31 | 32) {
            self.projection_receipts.insert(0x0f);
        }
    }
    pub(crate) fn framebuffer_dispatch_skipped(&mut self) {
        //4284B0 case15 capture/layout/skip guards have no further body
        //consumer. A preceding422680 Finish and kind2's explicit425B90
        //have their own acknowledged queue/raw projection hooks.
        //Such a guard must not invalidate prior acknowledged queue state.
        self.projection_receipts.insert(0x0f);
    }
    pub(crate) fn framebuffer_selected_finish_completed(&mut self) {
        //The default case15 path only calls422680 on its selected ten
        //actors. The caller has received Finish ACK and synchronized their
        //actual controls; unknown previous producers still prohibit receipt.
        if self.known_queues.is_some(){self.projection_receipts.insert(0x0f);}
    }
    ///425E60 partial effect reset keeps special66336/66341 producers.
    // The semantic adapter consumes native908 through paused_sprite_objects;
    //it omits primary/PCK advances without overwriting their retained rates.
    pub(crate) fn menu_pause_all(&mut self, paused: bool) {
        for sprite in &mut self.sprites {
            put_object_word(sprite, 908, u32::from(paused));
        }
    }
    ///444820's per-renderer pause is distinct from its retained rate at32.
    /// The semantic adapter uses this mask to omit shared actor advances;
    /// queued records and their rate are preserved while paused.
    pub(crate) fn paused_sprite_objects(&self)->BTreeSet<u16> {
        self.sprites.iter().enumerate().filter(|(_,record)|object_word(record,908)!=0).map(|(index,_)|index as u16).collect()
    }
    ///425F50's scene reset after406DB0. It does not apply432B90's
    ///menu-only zoom66512 reset.
    pub(crate) fn history_jump(&mut self,snapshot:&NativeSaveBody,selected:usize,page_run:usize,scene:&str)->Result<NativeSaveBody> {
        let mut body=NativeSaveBody::from_exact(&self.body)?;
        body.apply_log_metadata(snapshot,selected,page_run)?;
        self.body=body.bytes().to_vec();
        self.cstring(19336,128,scene)?;
        NativeSaveBody::from_exact(&self.body)
    }
    pub(crate) fn history_scene_entry(&mut self, target: &str) -> Result<()> {
        //433E50's15616 prelude clears these native audio declarations
        //before425F50/replay. Device playback is a separate owner fence.
        self.body[21944-LIVE_BASE]=0;
        self.body[265392-LIVE_BASE]=0;
        self.body[21984-LIVE_BASE..22112-LIVE_BASE].fill(0);
        self.cstring(19336, 128, target)?;
        self.write32(22132, 3);
        for address in [111844, 66508 * 4, 111848, 111852] { self.write32(address, 0); }
        for address in [266332, 265812, 265552] { self.body[address - LIVE_BASE] = 0; }
        Ok(())
    }
    ///432B90/425F50 menu entry after the distinct rollback/checkpoint copies.
    pub(crate) fn menu_entry(&mut self, target: &str, ui_exists: [bool; 2]) -> Result<()> {
        self.menu_pause_all(false);
        for address in [66510 * 4, 66512 * 4, 111844, 66508 * 4, 111848, 111852] {
            self.write32(address, 0);
        }
        self.cstring(19336, 128, target)?;
        self.write32(22132, 3);
        self.body[266332 - LIVE_BASE] = 0;
        if ui_exists[0] {
            self.body[265812 - LIVE_BASE] = 0;
        }
        if ui_exists[1] {
            self.body[265552 - LIVE_BASE] = 0;
        }
        self.projection_receipts.insert(0x3e);
        Ok(())
    }
    ///425DC0(0),423070,425B90(0), after the actual engine finish fence.
    pub(crate) fn movie_screen_clear(
        &mut self,
        protected: bool,
        animations_finalized: bool,
    ) -> Result<()> {
        if !animations_finalized && !self.native_primary_queues_idle()? {
            self.gap("52 native all-sprite finalization not acknowledged");
        }
        if animations_finalized {
            self.primary_queues.clear();
            self.auxiliary_queues.clear();
            if self.known_queues.is_some() {
                self.known_queues = Some(BTreeSet::new());
            }
        }
        for (index, sprite) in self.sprites.iter_mut().enumerate() {
            put_object_word(sprite, 740, 0);
            put_object_word(sprite, 908, 0);
            put_object_word(sprite, 904, if index == 0 { 0 } else { 50 });
        }
        self.write32(66510 * 4, 0);
        if !protected {
            for slot in 0..51 {
                if !(37..=44).contains(&slot) {
                    self.unload_image(slot)?;
                }
            }
        }
        if self.read32(66336 * 4) != 0 {
            self.unload_image(26)?;
        }
        if self.read32(66341 * 4) != 0 {
            self.unload_image(27)?;
        }
        for index in [66336, 66337, 66339, 66340, 66341, 66342] {
            self.write32(index * 4, 0);
        }
        self.projection_receipts.insert(0x52);
        Ok(())
    }
    pub(crate) fn framebuffer_color_effect(&mut self, kind: i16) {
        if matches!(kind, 0 | 1 | 2 | 3 | 9 | 15 | 16 | 17 | 20 | 21) {
            self.projection_receipts.insert(0x0f);
        }
    }
    pub(crate) fn framebuffer_fadeout_complete(&mut self, protected: bool) -> Result<()> {
        for (object, sprite) in self.sprites.iter_mut().enumerate() {
            put_object_word(sprite, 904, if object == 0 { 0 } else { 50 });
        }
        if !protected {
            for slot in 0..51 {
                if !(37..=44).contains(&slot) {
                    self.unload_image(slot)?;
                }
            }
        }
        self.framebuffer_falling_clear()?;
        Ok(())
    }
    pub(crate) fn framebuffer_falling_clear(&mut self) -> Result<()> {
        if self.read32(66336 * 4) != 0 {
            self.unload_image(26)?;
        }
        if self.read32(66341 * 4) != 0 {
            self.unload_image(27)?;
        }
        for index in [66336, 66337, 66339, 66340, 66341, 66342] {
            self.write32(index * 4, 0);
        }
        Ok(())
    }
    pub(crate) fn framebuffer_partial_clear(&mut self) {
        for index in [66337, 66339, 66340, 66342] {
            self.write32(index * 4, 0);
        }
    }
    pub(crate) fn framebuffer_queue_effect(&mut self, kind: i16) {
        if matches!(kind, 4 | 5 | 6 | 51 | 52) && self.known_queues.is_some() {
            self.projection_receipts.insert(0x0f);
        }
    }
    pub(crate) fn framebuffer_without_color(&mut self) {
        if self.known_queues.as_ref().is_some_and(|active| {
            ![10, 20, 30, 40, 50, 12, 22, 32, 42, 52]
                .into_iter()
                .any(|object| active.contains(&object))
        }) {
            self.projection_receipts.insert(0x0f);
        }
    }
    ///431F20 acknowledged resource-pointer transfer. Native types1..10 retain
    /// the source filename even after moving its live pointer;40EA70 marks CG
    /// preference data and does not clear that string.
    pub(crate) fn transfer_image(
        &mut self,
        source: u8,
        target: u8,
        clear_source_name: bool,
        dimensions: Option<([u32; 2], [u32; 2])>,
    ) -> Result<()> {
        ensure!(
            source < 51 && target < 51,
            "Circus native resource transfer slots"
        );
        let body = NativeSaveBody::from_exact(&self.body)?;
        let name = body.native_string(19496 + usize::from(source) * 48, 48)?;
        self.cstring(19496 + usize::from(target) * 48, 48, &name)?;
        self.image_bindings.remove(&source);
        for bound in self
            .object_resources
            .as_mut()
            .context("Circus object resource ownership unavailable")?
            .values_mut()
        {
            if *bound == source {
                *bound = target;
            }
        }
        let held = self
            .held_images
            .as_mut()
            .context("Circus native image ownership unavailable")?;
        held.remove(&source);
        held.insert(target);
        if clear_source_name {
            self.body[19496 - LIVE_BASE + usize::from(source) * 48] = 0;
        }
        if let Some((allocation, logical)) = dimensions {
            self.attach_image(target, allocation, logical)?;
        } else if sprite_for_slot(target).is_some() {
            self.gap("native resource transfer missing completed image dimension receipt");
        }
        Ok(())
    }
    ///4284B0 case60: ordinary resources are staged outside the save body.
    /// Slots19..23/26/27 are immediate resource calls and require their own
    /// completed422CF0/422DE0 publication before this projection can be admitted.
    pub(crate) fn stage_image(&mut self, slot: u8, text: &str) -> Result<()> {
        self.validate()?;
        ensure!(slot < 51, "Circus native staged image slot");
        if slot == 0 {
            self.projection_receipts.insert(0x3c);
            return Ok(());
        }
        ensure!(
            !text.starts_with('*'),
            "Circus native indirect staged image name"
        );
        if slot == 1 {
            if text.contains("BG\\BLACK") || text.contains("BG\\WHITE") {
                self.write32(66512 * 4, 0);
            }
            if self.read32(66512 * 4) == 0 {
                if [10, 20, 30, 40, 50].into_iter().any(|n| {
                    let s = &self.sprites[n];
                    object_float(s, 736) != 100.
                        || object_float(s, 36) != 1.
                        || object_word(s, 740) != 0
                }) {
                    self.gap(
                        "3c background requires original queued sprite-animation finalization",
                    );
                }
                //445010 resets the root even after native finalization. No
                // later state is substituted for the original constructor.
                self.sprites[0] = fresh_sprite();
                if let Some(refs) = self.references.as_mut() {
                    refs[0] = NativeReference {
                        position: [0., 0.],
                        scale: [100., 100.],
                    };
                }
                self.image_bindings.retain(|_, n| *n != 0);
                self.object_resources
                    .as_mut()
                    .context("Circus object resource ownership unavailable")?
                    .remove(&0);
            }
        }
        if matches!(slot, 19..=23 | 26 | 27) {
            self.gap(format!(
                "3c immediate resource slot{slot} requires native422DE0 name/publication policy"
            ));
        }
        self.projection_receipts.insert(0x3c);
        Ok(())
    }
    /// Native41F1B0 releases the actual resource then clears only its first
    /// filename byte.443FF0 detaches saved geometry only from bound objects.
    pub(crate) fn unload_image(&mut self, slot: u8) -> Result<()> {
        ensure!(slot < 51, "Circus native unload image slot");
        let name = 19496 - LIVE_BASE + usize::from(slot) * 48;
        if self
            .held_images
            .as_mut()
            .context("Circus native image ownership unavailable")?
            .remove(&slot)
        {
            self.body[name] = 0;
            self.image_bindings.remove(&slot);
            let aliases = self
                .object_resources
                .as_mut()
                .context("Circus object resource ownership unavailable")?;
            let detached = aliases
                .iter()
                .filter_map(|(object, bound)| (*bound == slot).then_some(*object))
                .collect::<Vec<_>>();
            aliases.retain(|_, bound| *bound != slot);
            for object in detached {
                for at in [64, 68, 184, 188] {
                    put_object_word(&mut self.sprites[usize::from(object)], at, 0);
                }
            }
            if (4..=8).contains(&slot) {
                self.unload_image(slot + 5)?;
            }
        }
        Ok(())
    }
    fn detach_sprite(&mut self, index: usize) {
        //445390 owns only queues outside the saved8..968 byte image.445A10
        // then applies these exact saved fields, retaining all other bytes.
        let sprite = &mut self.sprites[index];
        for at in [64, 68, 152, 156, 184, 188, 960] {
            put_object_word(sprite, at, 0);
        }
        for at in [116, 120] {
            put_object_float(sprite, at, 1.);
        }
        self.image_bindings.retain(|_, n| *n != index);
        if let Some(aliases) = &mut self.object_resources {
            aliases.remove(&(index as u16));
        }
    }
    ///4284B0 case73 :n/:r changes the protection field without425C20.
    pub(crate) fn background_protection(&mut self, protected: bool) {
        self.write32(66512 * 4, u32::from(protected));
        self.projection_receipts.insert(0x49);
    }
    ///425C20 deterministic saved fields after the engine has acknowledged the
    /// native animation finalization fence. A missing fence remains a gap.
    pub(crate) fn reset_stage(&mut self, animations_finalized: bool) -> Result<()> {
        self.validate()?;
        let selected = |object: u16| matches!(object, 3 | 4 | 5 | 10..=63 | 66 | 88);
        let known_idle = self
            .known_queues
            .as_ref()
            .is_some_and(|active| !active.iter().copied().any(selected));
        if !animations_finalized && !known_idle {
            self.gap("49 native queued animation finalization not acknowledged");
        }
        if animations_finalized {
            self.primary_queues.retain(|object| !selected(*object));
            self.auxiliary_queues.retain(|object| !selected(*object));
            if let Some(active) = self.known_queues.as_mut() {
                active.retain(|object| !selected(*object));
            }
        }
        for slot in 4..=13 {
            self.unload_image(slot)?;
        }
        if self.read32(66341 * 4) != 0 {
            self.write32(66341 * 4, 0);
            self.unload_image(27)?;
        }
        self.unload_image(25)?;
        self.sprites[63] = fresh_sprite();
        if let Some(refs) = self.references.as_mut() {
            refs[63] = NativeReference {
                position: [0., 0.],
                scale: [100., 100.],
            };
        }
        self.image_bindings.retain(|_, index| *index != 63);
        for index in [3, 4, 5, 88] {
            self.detach_sprite(index);
        }
        self.unload_image(31)?;
        self.unload_image(32)?;
        for s in &mut self.sprites[..60] {
            put_object_word(s, 960, 0);
        }
        self.write32(66517 * 4, 0);
        put_object_word(&mut self.sprites[0],964,0);
        if self.read32(66512 * 4) == 0 {
            self.write32(66513 * 4, 0);
        }
        self.remember_position(0);
        //457F90 finalizes active Movie queues; their complete1668-byte native
        // record must come from that backend, never an invented empty record.
        self.projection_receipts.insert(0x49);
        Ok(())
    }
    ///4284B0 case79/80 plus425DC0/425F50. UI existence controls the two
    /// conditional text clears, supplied from the semantic adapter.
    pub(crate) fn transfer_scene(
        &mut self,
        opcode: u8,
        target: &str,
        previous_index: i32,
        ui_exists: [bool; 2],
        protected: bool,
        animations_finalized: bool,
    ) -> Result<()> {
        self.validate()?;
        ensure!(
            matches!(opcode, 0x4f | 0x50),
            "Circus native scene transfer opcode"
        );
        if opcode == 0x4f {
            self.body[265392 - LIVE_BASE] = 0;
            self.body[21984 - LIVE_BASE..21984 - LIVE_BASE + 128].fill(0);
        }
        if opcode == 0x50 || previous_index >= 0 {
            self.record_scene_bank_changes()?;
            if opcode == 0x4f {
                self.write32(27965 * 4, self.read32(27964 * 4));
                self.write32(27967 * 4, self.read32(27966 * 4));
            }
        }
        //Both opcodes clear these filename bytes before425F50, including
        //the retained-resource mode used by50. Do not erase trailing bytes.
        for slot in 0..51 {
            if !(37..=44).contains(&slot) {
                self.body[19496 - LIVE_BASE + slot * 48] = 0;
            }
        }
        if opcode == 0x4f {
            let known_idle = self.known_queues.as_ref().is_some_and(BTreeSet::is_empty);
            if !animations_finalized && !known_idle {
                self.gap("4f native queued animation finalization not acknowledged");
            }
            //425DC0 calls44A4C0 before40EF40(0): a renderer whose908
            //pause bit was set keeps its queues, then becomes unpaused. The
            //caller has synchronized actual actor states after that fence;
            //preserve those maps and raw activation fields, not a fake empty
            //queue set merely because the native finalization call completed.
            for s in &mut self.sprites {
                put_object_word(s, 740, 0);
            }
        }
        for s in &mut self.sprites {
            put_object_word(s, 908, 0);
        }
        self.write32(66510 * 4, 0);
        if opcode == 0x4f {
            for (index, s) in self.sprites.iter_mut().enumerate() {
                put_object_word(s, 904, if index == 0 { 0 } else { 50 });
            }
            if !protected {
                //41F1B0 tests live handles, not already-cleared filenames.
                let loaded: Vec<_> = self
                    .held_images
                    .as_ref()
                    .context("Circus native image ownership unavailable")?
                    .iter()
                    .copied()
                    .filter(|slot| !(37..=44).contains(slot))
                    .collect();
                for slot in loaded {
                    self.unload_image(slot)?;
                }
            }
            if self.read32(66336 * 4) != 0 {
                self.unload_image(26)?;
            }
            if self.read32(66341 * 4) != 0 {
                self.unload_image(27)?;
            }
            for word in [66336, 66337, 66339, 66340, 66341, 66342] {
                self.write32(word * 4, 0);
            }
        }
        self.cstring(19336, 128, target)?;
        self.write32(22132, 3);
        self.write32(111844, 0);
        self.write32(66508 * 4, 0);
        self.body[266332 - LIVE_BASE] = 0;
        if ui_exists[0] {
            self.body[265812 - LIVE_BASE] = 0;
        }
        if ui_exists[1] {
            self.body[265552 - LIVE_BASE] = 0;
        }
        self.write32(111848, 0);
        self.write32(111852, 0);
        self.projection_receipts.insert(opcode);
        Ok(())
    }
    pub(crate) fn position(&mut self, slot: u8, position: [i16; 2]) {
        if let Some(index) = sprite_for_slot(slot) {
            let s = &mut self.sprites[index];
            for (at, value) in [
                (8, position[0] as f32),
                (12, position[1] as f32),
                (56, position[0] as f32),
                (60, position[1] as f32),
            ] {
                put_object_float(s, at, value);
            }
            if matches!(slot, 4..=8 | 25) {
                self.remember_position(index);
            }
        }
    }
    ///401040/444040 direct object coordinates.4440E0's remembered968/972
    /// fields are outside the copied8..968 record and are not fabricated here.
    pub(crate) fn object_position(
        &mut self,
        opcode: u8,
        object: u16,
        position: [i32; 2],
    ) -> Result<()> {
        ensure!(
            object < 150 && matches!(opcode, 0x89 | 0x94 | 0xa1),
            "Circus native direct object position opcode/index"
        );
        let s = &mut self.sprites[usize::from(object)];
        for (at, value) in [
            (8, position[0]),
            (12, position[1]),
            (56, position[0]),
            (60, position[1]),
        ] {
            put_object_float(s, at, value as f32);
        }
        if opcode != 0xa1 {
            self.remember_position(usize::from(object));
        }
        self.projection_receipts.insert(opcode);
        Ok(())
    }
    ///444790 controls native10..63 painter priority, not pixel alpha.
    pub(crate) fn portrait_pose(
        &mut self,
        object: u16,
        position: Option<[i32; 2]>,
        scale: Option<(i16, [i16; 2])>,
    ) -> Result<()> {
        ensure!(object < 150, "Circus portrait object index");
        let sprite = &mut self.sprites[usize::from(object)];
        if let Some([x, y]) = position {
            for (at, value) in [
                (8, x as f32),
                (12, y as f32),
                (56, x as f32),
                (60, y as f32),
            ] {
                put_object_float(sprite, at, value);
            }
        }
        if let Some((percent, pivot)) = scale {
            for (at, value) in [
                (128, percent as f32),
                (132, percent as f32),
                (152, pivot[0] as f32),
                (156, pivot[1] as f32),
            ] {
                put_object_float(sprite, at, value);
            }
        }
        if position.is_some() {
            self.remember_position(usize::from(object));
        }
        if scale.is_some() {
            self.remember_scale(usize::from(object));
        }
        Ok(())
    }
    pub(crate) fn object_priority(&mut self, object: u16, priority: i16) -> Result<()> {
        ensure!(object < 150, "Circus native painter object index");
        put_object_word(
            &mut self.sprites[usize::from(object)],
            904,
            i32::from(priority).clamp(0, 100) as u32,
        );
        self.projection_receipts.insert(0x96);
        Ok(())
    }
    /// Native4472A0/4475C0/4476C0/4477D0 immediately retain target fields.
    /// Their update/queue records still require exact native projection; a
    /// shared animation's presence alone cannot certify those saved bytes.
    pub(crate) fn object_track_target(
        &mut self,
        opcode: u8,
        object: u16,
        kind: u8,
        values: [f32; 2],
    ) -> Result<()> {
        ensure!(
            object < 150 && values.iter().all(|v| v.is_finite()),
            "Circus native track object/scalars"
        );
        let s = &mut self.sprites[usize::from(object)];
        match kind {
            0 => {
                put_object_float(s, 56, values[0]);
                put_object_float(s, 60, values[1]);
            }
            1 => put_object_float(s, 136, values[0]),
            2 => put_object_float(s, 148, values[0]),
            3 => put_object_float(s, 124, values[0]),
            _ => anyhow::bail!("Circus native track kind"),
        }
        self.gap(format!(
            "opcode0x{opcode:02x} native queued track/update save fields not projected"
        ));
        Ok(())
    }
    ///426300 copies these seven retained face-name records into downloaded
    ///bodies; the independent19328-byte preference prefix cannot supply them.
    pub(crate) fn cloud_face_names(&self)->Vec<u8> {
        self.body[21272-LIVE_BASE..21608-LIVE_BASE].to_vec()
    }
    pub(crate) fn retained_music_name(&self)->Result<String> {
        NativeSaveBody::from_exact(&self.body)?.native_string(21944,32)
    }
    pub(crate) fn snapshot_object_record(&self,object:u16)->Result<Vec<u8>> {
        ensure!(object<150,"Circus native snapshot object index");
        Ok(NativeSaveBody::from_exact(&self.body)?.sprite(usize::from(object))?.bytes().to_vec())
    }
    ///444020 assigns renderer DWORD46 directly; the sprite body starts+8.
    pub(crate) fn resource_visible(&mut self,object:u16,visible:bool)->Result<()> {
        ensure!(object<150,"Circus native visible object index");
        put_object_word(&mut self.sprites[usize::from(object)],184,u32::from(visible));
        Ok(())
    }
    pub(crate) fn reset_object(&mut self, object: u16) -> Result<()> {
        ensure!(object < 150, "Circus reset object index");
        self.sprites[usize::from(object)] = fresh_sprite();
        self.image_bindings
            .retain(|_, bound| *bound != usize::from(object));
        self.object_resources
            .as_mut()
            .context("Circus object resource ownership unavailable")?
            .remove(&object);
        if let Some(references) = &mut self.references {
            references[usize::from(object)] = NativeReference {
                position: [0.; 2],
                scale: [100.; 2],
            };
        }
        self.blur_activation.remove(&object);
        self.primary_ieee.remove(&object);
        Ok(())
    }
    pub(crate) fn sprite_reset_seed(object: u16) -> Result<crate::machine::sprite_ops::SpriteSeed> {
        ensure!(object < 150, "Circus reset seed object index");
        let mut values = vec![0.; 17];
        for index in [2, 6, 7, 10, 11, 16] {
            values[index] = 1.;
        }
        values[3] = 100.;
        values[4] = 100.;
        let mut retained = values.clone();
        for index in [2, 3, 4] {
            retained[index] = 0.;
        }
        Ok(crate::machine::sprite_ops::SpriteSeed {
            object,
            values,
            retained,
        })
    }
    pub(crate) fn sprite_seed(
        &self,
        object: u16,
    ) -> Result<crate::machine::sprite_ops::SpriteSeed> {
        ensure!(object < 150, "Circus sprite seed index");
        let s = &self.sprites[usize::from(object)];
        let mut values = [
            8, 12, 112, 128, 132, 144, 36, 40, 44, 48, 116, 120, 152, 156, 104, 108, 32,
        ]
        .into_iter()
        .map(|offset| object_float(s, offset))
        .collect::<Vec<_>>();
        values[14] -= object_float(s, 96);
        values[15] -= object_float(s, 100);
        let mut retained = values.clone();
        for (index, offset) in [(0, 56), (1, 60), (2, 124), (3, 136), (4, 136), (5, 148)] {
            retained[index] = object_float(s, offset);
        }
        for (channel,value) in values.iter().enumerate() {
            ensure!(value.is_finite()||self.primary_ieee.get(&object).and_then(|proof|proof.get(&(channel as u8)))==Some(&value.to_bits()),
                "Circus nonfinite sprite seed lacks acknowledged zero-divisor sample");
        }
        for (channel,value) in retained.iter().enumerate() {
            ensure!(value.is_finite()||self.primary_ieee.get(&object).and_then(|proof|proof.get(&(channel as u8)))==Some(&value.to_bits()),
                "Circus nonfinite retained seed lacks acknowledged IEEE sample");
        }
        Ok(crate::machine::sprite_ops::SpriteSeed {
            object,
            values,
            retained,
        })
    }
    pub(crate) fn blur_immediate(&mut self, denominator: i16) -> Result<()> {
        ensure!((0..=1000).contains(&denominator), "Circus blur denominator");
        for s in self.sprites.iter_mut().take(60) {
            if denominator == 0 || denominator == 1000 {
                put_object_word(s, 960, 0);
            } else {
                put_object_word(s, 960, 4);
                put_object_float(s, 116, 1. / denominator as f32);
                put_object_float(s, 120, 1. / denominator as f32);
            }
        }
        Ok(())
    }
    pub(crate) fn packed_frame_attached(
        &mut self,
        object: u16,
        allocation: [u32; 2],
        logical: [u32; 2],
    ) -> Result<()> {
        self.bind_resource(33, object, None, Some((allocation, logical)))?;
        // The pack archive and its decoded frame are separate native leases;
        //41F1B0(33) releases the archive without443FF0-detaching each frame.
        self.object_resources
            .as_mut()
            .context("Circus packed frame ownership unavailable")?
            .remove(&object);
        self.image_bindings
            .retain(|_, bound| *bound != usize::from(object));
        Ok(())
    }
    ///Case60's packed-loader path copies the original filename verbatim,
    ///unlike422CF0 raster loading which uppercases its local buffer.
    pub(crate) fn packed_resource_loaded(&mut self, slot: u8, name: &str) -> Result<()> {
        ensure!((33..=36).contains(&slot), "Circus packed resource slot");
        self.unload_image(slot)?;
        self.cstring(19496 + usize::from(slot) * 48, 48, name)?;
        self.held_images.as_mut().context("Circus packed resource ownership")?.insert(slot);
        self.projection_receipts.insert(0x3c);
        Ok(())
    }
    pub(crate) fn stage_opcode_completed(&mut self, opcode: u8, noop: bool) -> Result<()> {
        ensure!(matches!(opcode, 0x28 | 0x88 | 0x9a..=0x9d), "Circus stage opcode receipt");
        //9D's declaration writes66512/66513 through stage_pending; its
        //422650 prelude synchronizes primary and auxiliary controls before
        //this receipt. An earlier unacknowledged producer remains unknown.
        //In particular, legal0F skip guards must carry their own receipt,
        //rather than erroneously poisoning this otherwise known stage state.
        if opcode == 0x88 || opcode == 0x28 || noop || self.known_queues.is_some() {
            self.projection_receipts.insert(opcode);
        }
        Ok(())
    }
    ///447AC0's looping descriptor.920 is a relocated process reference in
    ///original files; emitted images retain a client-scoped immutable pack
    ///lineage token, never a current-slot alias or device/address value.
    ///425640 rebinds imported descriptors from the saved pack filename.
    pub(crate) fn packed_declared(
        &mut self,
        object: u16,
        slot: u8,
        frames: u32,
        hold_ticks: i16,
        looping: bool,
        lineage:u32,
    ) -> Result<()> {
        ensure!(
            object < 150 && (33..=36).contains(&slot) && frames > 0 && lineage!=0,
            "Circus packed declaration receipt"
        );
        if looping {
            let sprite = &mut self.sprites[usize::from(object)];
            for offset in (912..960).step_by(4) {
                put_object_word(sprite, offset, 0);
            }
            for (offset, value) in [
                (920, lineage),
                (924, hold_ticks as i32 as u32),
                (932, frames),
                (956, 1),
            ] {
                put_object_word(sprite, offset, value);
            }
        }
        Ok(())
    }
    pub(crate) fn sprite_opcode_completed(&mut self, opcode: u8) {
        self.projection_receipts.insert(opcode);
    }
    pub(crate) fn background_protection_opcode(
        &mut self,
        opcode: u8,
        protected: bool,
    ) -> Result<()> {
        ensure!(
            matches!(opcode, 0x49 | 0x4a),
            "Circus background protection opcode"
        );
        self.write32(66512 * 4, u32::from(protected));
        self.projection_receipts.insert(opcode);
        Ok(())
    }
    pub(crate) fn sprite_sequence_status(
        &mut self,
        object: u16,
        status: &crysirror_scene::layer::queued::SequenceStatus,
    ) -> Result<()> {
        use crysirror_scene::layer::queued::{Active, SequenceTrack};
        ensure!(
            object < 150 && status.values.len() == 17 && status.retained.len() == 17,
            "Circus primary sequence object/channels"
        );
        let mut ieee=BTreeMap::new();
        for (channel,value) in status.values.iter().enumerate() {
            if !value.is_finite() {
                ensure!(status.allows_nonfinite_channel(channel as u8),"Circus primary sample lacks zero-divisor provenance");
                ieee.insert(channel as u8,value.to_bits());
            }
        }
        if ieee.is_empty(){self.primary_ieee.remove(&object);}else{self.primary_ieee.insert(object,ieee);}
        let s = &mut self.sprites[usize::from(object)];
        for (index, offset) in [
            (0, 8),
            (1, 12),
            (2, 112),
            (3, 128),
            (4, 132),
            (5, 144),
            (10, 116),
            (11, 120),
            (12, 152),
            (13, 156),
            (16, 32),
        ] {
            put_object_float(s, offset, status.values[index]);
        }
        for (index, offset) in [(0, 56), (1, 60), (2, 124), (3, 136), (5, 148)] {
            put_object_float(s, offset, status.retained[index]);
        }
        for lane_id in [0u8, 1, 2, 3, 4, 6, 7, 8, 9, 10] {
            let active_offset = 632 + usize::from(lane_id) * 4;
            let cursor_offset = 676 + usize::from(lane_id) * 4;
            let Some(lane) = status.lanes.get(&lane_id) else {
                continue;
            };
            put_object_word(s, active_offset, u32::from(lane.active.is_some()));
            put_object_word(s, cursor_offset, u32::from(lane.cursor));
            let (Some(record), Some(active)) = (&lane.last_record, &lane.last_active) else {
                continue;
            };
            match (&record.track, active, lane_id) {
                (
                    SequenceTrack::Linear { target, .. },
                    Active::Linear {
                        velocity,
                        multiplier,
                    },
                    0,
                ) => {
                    ensure!(
                        target.len() == 2 && velocity.len() == 2,
                        "Circus move queue arity"
                    );
                    for (offset, value) in [
                        (208, target[0]),
                        (212, target[1]),
                        (216, velocity[0]),
                        (220, velocity[1]),
                        (228, *multiplier),
                    ] {
                        put_object_float(s, offset, value);
                    }
                }
                (
                    SequenceTrack::Linear { target, .. },
                    Active::Linear {
                        velocity,
                        multiplier,
                    },
                    3 | 4 | 6,
                ) => {
                    ensure!(
                        !target.is_empty() && !velocity.is_empty(),
                        "Circus scalar queue arity"
                    );
                    let offsets = match lane_id {
                        3 => [316, 344, 348],
                        4 => [356, 384, 388],
                        _ => [436, 464, 468],
                    };
                    for (offset, value) in
                        offsets
                            .into_iter()
                            .zip([target[0], velocity[0], *multiplier])
                    {
                        put_object_float(s, offset, value);
                    }
                }
                (
                    SequenceTrack::Oscillation {
                        amplitude,
                        end_degrees,
                        ..
                    },
                    Active::Oscillation {
                        phase,
                        velocity,
                        baseline,
                        multiplier,
                    },
                    1 | 2,
                ) => {
                    let offsets = if lane_id == 1 {
                        [232, 236, 248, 256, 260, 268]
                    } else {
                        [272, 276, 292, 300, 296, 308]
                    };
                    for (offset, value) in offsets.into_iter().zip([
                        *phase,
                        *end_degrees,
                        *baseline,
                        *velocity,
                        *amplitude,
                        *multiplier,
                    ]) {
                        put_object_float(s, offset, value);
                    }
                }
                (
                    SequenceTrack::Linear { target, start, .. },
                    Active::Linear {
                        velocity,
                        multiplier,
                    },
                    10,
                ) => {
                    ensure!(
                        target.len() == 2
                            && velocity.len() == 2
                            && start.as_ref().is_some_and(|s| s.len() == 2),
                        "Circus blur queue arity"
                    );
                    let raw = if target[0] == 0. {
                        0.
                    } else {
                        (1. / target[0]).round()
                    };
                    for (offset, value) in [
                        (596, target[0]),
                        (612, raw),
                        (624, velocity[0]),
                        (628, *multiplier),
                    ] {
                        put_object_float(s, offset, value);
                    }
                    let signature = serde_json::to_vec(record)?;
                    let busy = lane.active.is_some();
                    let previous = self.blur_activation.get(&object);
                    if lane.records.is_empty()
                        && raw == 1000.
                        && previous.is_none_or(|(seen, was_busy)| *seen != signature || *was_busy)
                    {
                        put_object_word(s, 960, 0);
                    } else if busy
                        && previous.is_none_or(|(seen, was_busy)| *seen != signature || !*was_busy)
                    {
                        put_object_word(s, 960, 4);
                    }
                    self.blur_activation.insert(object, (signature, busy));
                }
                (SequenceTrack::Frames { .. }, Active::Frames { .. }, 9) => {}
                (SequenceTrack::Frames { .. }, Active::Frames { .. }, 7 | 8) => {}
                _ => anyhow::bail!("Circus primary queue track/lane mismatch"),
            }
        }
        if status.lanes.values().any(|lane| !lane.records.is_empty()) {
            self.primary_queues.insert(object);
        } else {
            self.primary_queues.remove(&object);
        }
        if let Some(active) = self.known_queues.as_mut() {
            *active = self
                .primary_queues
                .union(&self.auxiliary_queues)
                .copied()
                .collect();
        }
        Ok(())
    }
    pub(crate) fn scale(&mut self, slot: u8, percent: i16, pivot: [i16; 2]) {
        if let Some(index) = sprite_for_slot(slot) {
            let s = &mut self.sprites[index];
            for (at, value) in [
                (128, percent as f32),
                (132, percent as f32),
                (152, pivot[0] as f32),
                (156, pivot[1] as f32),
            ] {
                put_object_float(s, at, value);
            }
            self.remember_scale(index);
        }
    }
    pub(crate) fn clear_resource_selectors(&mut self, object: u16) -> Result<()> {
        ensure!(object < 150, "Circus444750 object index");
        self.sprites[usize::from(object)][904..952].fill(0);
        Ok(())
    }
    ///4284B0 case55 only sets dispatcher-local v570[133]. A checkpoint
    ///before the resource tail must retain the old sprite coordinates; the
    ///actual publication later calls center_resource after its engine ACK.
    pub(crate) fn resource_center_declared(&mut self) {
        self.projection_receipts.insert(0x37);
    }
    pub(crate) fn center_resource(&mut self, object: u16, position: [i32; 2]) -> Result<()> {
        ensure!(object < 150, "Circus resource centering object");
        for (at, value) in [
            (8, position[0]),
            (12, position[1]),
            (56, position[0]),
            (60, position[1]),
        ] {
            put_object_float(&mut self.sprites[usize::from(object)], at, value as f32);
        }
        self.remember_position(usize::from(object));
        self.projection_receipts.insert(0x37);
        Ok(())
    }
    pub(crate) fn resource_texture_offset(
        &mut self,
        objects: &[u16],
        offset: [f32; 2],
    ) -> Result<()> {
        ensure!(
            objects.iter().all(|n| *n < 150) && offset.iter().all(|v| v.is_finite()),
            "Circus native texture offset"
        );
        for object in objects {
            let sprite = &mut self.sprites[usize::from(*object)];
            put_object_float(sprite, 20, offset[0]);
            put_object_float(sprite, 24, offset[1]);
        }
        Ok(())
    }
    pub(crate) fn completed_resource_declaration(&mut self, slot: u8) -> Result<()> {
        ensure!(
            matches!(slot, 19..=23 | 26 | 27),
            "Circus immediate declaration slot"
        );
        self.projection_receipts.insert(0x3c);
        Ok(())
    }
    pub(crate) fn resource_opcode_completed(&mut self, opcode: u8) -> Result<()> {
        ensure!(
            matches!(opcode, 0x3d | 0x3f | 0x40 | 0x41 | 0x49 | 0x4a),
            "Circus resource opcode receipt"
        );
        self.projection_receipts.insert(opcode);
        Ok(())
    }
    pub(crate) fn object_record(&self, object: u16) -> Result<Vec<u8>> {
        ensure!(object < 150, "Circus native object record index");
        Ok(self.sprites[usize::from(object)].clone())
    }
    ///447D20 copies the source record; reusable actor reconstruction and binding
    /// acknowledgments remain the caller's prerequisite, never inferred here.
    pub(crate) fn restore_object_record(&mut self, object: u16, record: &[u8]) -> Result<()> {
        ensure!(
            object < 150 && record.len() == 960,
            "Circus native restored object extent"
        );
        self.sprites[usize::from(object)].copy_from_slice(record);
        Ok(())
    }
    ///443EA0 + optional444120. Slot ownership and drawable aliases are distinct.
    pub(crate) fn bind_resource(
        &mut self,
        slot: u8,
        object: u16,
        crop: Option<[i32; 4]>,
        dimensions: Option<([u32; 2], [u32; 2])>,
    ) -> Result<()> {
        ensure!(
            slot < 51 && object < 150,
            "Circus native resource binding index"
        );
        if dimensions.is_some() {
            self.held_images
                .as_mut()
                .context("Circus native resource ownership unavailable")?
                .insert(slot);
        }
        let aliases = self
            .object_resources
            .as_mut()
            .context("Circus object resource ownership unavailable")?;
        let sprite = &mut self.sprites[usize::from(object)];
        if let Some((allocation, logical)) = dimensions {
            ensure!(
                allocation.iter().chain(&logical).all(|v| *v > 0),
                "Circus native bound image extent"
            );
            aliases.insert(object, slot);
            for (at, value) in [
                (168, allocation[0]),
                (172, allocation[1]),
                (72, logical[0]),
                (76, logical[1]),
                (176, allocation[0]),
                (180, allocation[1]),
            ] {
                put_object_float(sprite, at, value as f32);
            }
            if object_float(sprite, 104) == object_float(sprite, 96)
                || object_float(sprite, 108) == object_float(sprite, 100)
            {
                for (at, value) in [
                    (96, 0.),
                    (100, 0.),
                    (104, logical[0] as f32),
                    (108, logical[1] as f32),
                ] {
                    put_object_float(sprite, at, value);
                }
            }
            for (at, value) in [(64, allocation[0]), (68, allocation[1])] {
                if object_float(sprite, at) == 0. {
                    put_object_float(sprite, at, value as f32);
                }
            }
            put_object_word(sprite, 184, 1);
        } else {
            aliases.remove(&object);
            for at in [168, 172, 72, 76, 188] {
                put_object_word(sprite, at, 0);
            }
        }
        if let Some(crop) = crop {
            for (at, value) in [96, 100, 104, 108].into_iter().zip(crop) {
                put_object_float(sprite, at, value as f32);
            }
        }
        Ok(())
    }
    ///0F/9 binds the actual shared frame snapshot to renderer0. Its
    ///relocated image lease is not native resource-slot1 ownership.
    pub(crate) fn capture_background(&mut self) -> Result<()> {
        self.validate()?;
        let sprite = &mut self.sprites[0];
        for (offset, value) in [(168, 2048.), (172, 1024.), (72, 1280.), (76, 720.), (176, 2048.), (180, 1024.)] {
            put_object_float(sprite, offset, value);
        }
        if object_float(sprite, 104) == object_float(sprite, 96)
            || object_float(sprite, 108) == object_float(sprite, 100) {
            for (offset, value) in [(96, 0.), (100, 0.), (104, 1280.), (108, 720.)] {
                put_object_float(sprite, offset, value);
            }
        }
        for (offset, value) in [(64, 2048.), (68, 1024.)] {
            if object_float(sprite, offset) == 0. { put_object_float(sprite, offset, value); }
        }
        put_object_word(sprite, 184, 1);
        //Snapshot ownership lives in the client-scoped engine capability;
        //no device pointer is copied into the portable native image.
        put_object_word(sprite, 188, 0);
        self.image_bindings.retain(|_, object| *object != 0);
        self.object_resources.as_mut().context("Circus snapshot renderer ownership")?.remove(&0);
        Ok(())
    }
    ///443EA0 uses native texture-table allocation size and logical image size.
    /// These extents come from the completed engine-owned image resource.
    pub(crate) fn attach_image(
        &mut self,
        slot: u8,
        allocation: [u32; 2],
        logical: [u32; 2],
    ) -> Result<()> {
        self.validate()?;
        ensure!(
            allocation.iter().chain(logical.iter()).all(|v| *v > 0),
            "Circus native texture dimensions"
        );
        self.held_images
            .as_mut()
            .context("Circus native image ownership unavailable")?
            .insert(slot);
        let Some(index) = sprite_for_slot(slot) else {
            return Ok(());
        };
        let s = &mut self.sprites[index];
        for (at, value) in [
            (168, allocation[0]),
            (172, allocation[1]),
            (72, logical[0]),
            (76, logical[1]),
            (176, allocation[0]),
            (180, allocation[1]),
        ] {
            put_object_float(s, at, value as f32);
        }
        if object_float(s, 104) == object_float(s, 96)
            || object_float(s, 108) == object_float(s, 100)
        {
            for (at, value) in [
                (96, 0.),
                (100, 0.),
                (104, logical[0] as f32),
                (108, logical[1] as f32),
            ] {
                put_object_float(s, at, value);
            }
        }
        for (at, value) in [(64, allocation[0]), (68, allocation[1])] {
            if object_float(s, at) == 0. {
                put_object_float(s, at, value as f32);
            }
        }
        put_object_word(s, 184, 1);
        // Never retain a host/device identity in original native VM data.
        // The semantic binding is separate;41DD10 clears saved identity188.
        self.image_bindings.insert(slot, index);
        self.object_resources
            .as_mut()
            .context("Circus object resource ownership unavailable")?
            .insert(index as u16, slot);
        Ok(())
    }
    /// The accepted falling-particle profiles of4284B0 case40. Pool storage
    /// belongs to the shared VFX worker; these words are original save policy.
    ///425B90 preserves66338 (the previous intensity) when disabling effects.
    pub(crate) fn special_effects(&self) -> [bool; 2] {
        [self.read32(66336 * 4) != 0, self.read32(66341 * 4) != 0]
    }
    pub(crate) fn falling_effect(&mut self, kind: i16, parameter: i16) -> Result<()> {
        if kind == 0 {
            for index in [66336, 66337, 66339, 66340, 66341, 66342] {
                self.write32(index * 4, 0);
            }
        } else if matches!(kind, 2 | 7 | 13 | 15 | 16 | 17) {
            let field = match kind { 2 => 66339, 7 => 66336, 13 | 17 => 66340, 15 => 66342, 16 => 66341, _ => unreachable!() };
            self.write32(field * 4, if matches!(kind, 7 | 16) { u32::from(parameter != 0) } else { i32::from(parameter) as u32 });
        } else {
            self.write32(66337 * 4, i32::from(kind) as u32);
            self.write32(66338 * 4, i32::from(parameter) as u32);
        }
        Ok(())
    }
    pub(crate) fn radial_control(
        &mut self,
        origin: Option<[i32; 2]>,
        angles: Option<([i32; 2], i32)>,
    ) {
        if let Some(origin) = origin {
            self.write32(66343 * 4, origin[0] as u32);
            self.write32(66344 * 4, origin[1] as u32);
        }
        if let Some((angles, duration)) = angles {
            self.write32(66345 * 4, angles[0] as u32);
            self.write32(66346 * 4, angles[1] as u32);
            self.write32(66347 * 4, duration as u32);
        }
    }
    pub(crate) fn audit_completed(&mut self, opcode: u8, scene: &str, pc: u32) -> Result<()> {
        self.validate()?;
        self.completed.insert(opcode);
        let projected = self.projection_receipts.remove(&opcode);
        if !projected
            && matches!(
                opcode,
                0x0f | 0x36
                    | 0x38
                    | 0x3f
                    | 0x40
                    | 0x41
                    | 0x4a
                    | 0x52
                    | 0x89
                    | 0x8e
                    | 0x8f
                    | 0x95
                    | 0x9a
                    | 0x9b
                    | 0x9c
                    | 0x9d
                    | 0xa8
            )
        {
            self.known_queues = None;
        }
        self.cstring(19336, 128, scene)?;
        self.write32(22132, pc);
        // This deliberately narrow set has either a direct implementation
        // above or the original dispatch's explicit no-live-body branch.
        //4284B0 case97(0x61) only copies its callback into Main+11416.
        //That Main-heap owner is not the separately allocated Buf save body;
        //the callback's eventual PC changes use advance_pc like other calls.
        if !projected
            && !matches!(opcode,0x00..=0x04|0x06|0x08|0x0b|0x0d|0x0e|0x11|0x12|0x13|0x14|0x15|0x16|0x17|0x19|0x1a|0x1c|0x1f|0x20|0x23|0x24|0x25|0x27|
            0x29..=0x31|0x3a|0x3b|0x42|0x48|0x4b|0x53|0x54|0x57|0x5d|0x60|0x61|0x62|0x66|0x69|0x6c|0x6e|0x8b..=0x8d|
            0x32|0x34|0x35|0x43|0x46|0x47|0x51|0x63|0x65|0x68|0x70|0x72|0x73|0x76|0x78|0x7a|0x7b|0x82|0x83|0x85|0x86|0x87|0x90|0x91|0x92|0x93|0x97|0x98|0x99|0x9e|0xa3|0xa4|0xa5|0xa6|0xaa|0xae|0xaf|0xb0)
        {
            self.gap(format!(
                "opcode0x{opcode:02x} live-body writes not yet audited"
            ));
        }
        Ok(())
    }
    pub(crate) fn checkpoint(&mut self, resume: u32) -> Result<NativeSaveBody> {
        self.validate()?;
        self.write32(22132, resume);
        self.write32(22144, resume);
        if let Some(movie) = &self.movie {
            self.body[266340 - LIVE_BASE..268008 - LIVE_BASE].copy_from_slice(&movie.record);
        }
        self.copy_native_sprite_records()
    }
    fn copy_native_sprite_records(&mut self) -> Result<NativeSaveBody> {
        let at = 121248 - LIVE_BASE;
        for (index, sprite) in self.sprites.iter().enumerate() {
            self.body[at + index * SPRITE_BYTES..at + (index + 1) * SPRITE_BYTES]
                .copy_from_slice(sprite);
        }
        let mut image = NativeSaveBody::from_exact(&self.body)?;
        image.apply_checkpoint_sprite_fixups();
        //41DD10 fixes the live records before its memcpy, not only the mirror.
        self.body.copy_from_slice(image.bytes());
        Ok(image)
    }
    pub fn admitted_checkpoint(&mut self, resume: u32) -> Result<NativeSaveBody> {
        ensure!(
            self.gaps.is_empty(),
            "Circus native save projection gaps: {:?}",
            self.gaps
        );
        self.checkpoint(resume)
    }
    ///422B70 copies the current live image without41DD10's5536 write.
    ///Its caller has already finalized/unpaused the exact required actors.
    pub(crate) fn freeze_menu_checkpoint(&mut self)->Result<CapturedNativeBody> {
        self.validate()?;
        if let Some(movie)=&self.movie{self.body[266340-LIVE_BASE..268008-LIVE_BASE].copy_from_slice(&movie.record);}
        let body=self.copy_native_sprite_records()?;
        Ok(CapturedNativeBody::capture(body,self.gaps.clone()))
    }
    pub(crate) fn freeze_checkpoint(&mut self, resume: u32) -> Result<CapturedNativeBody> {
        let gaps = self.gaps.clone();
        let body = self.checkpoint(resume)?;
        Ok(CapturedNativeBody::capture(body, gaps))
    }
    pub(crate) fn freeze_dialogue_checkpoint(&mut self,resume:u32)->Result<CapturedNativeBody> {
        //41DD10 sets5536 to6B's start(Main24472);5533 retains the current
        //decoder next PC. These are independent native restore positions.
        self.write32(22144,resume);
        if let Some(movie)=&self.movie {self.body[266340-LIVE_BASE..268008-LIVE_BASE].copy_from_slice(&movie.record);}
        let body=self.copy_native_sprite_records()?;
        Ok(CapturedNativeBody::capture(body,self.gaps.clone()))
    }
    ///432AB0 menu/backlog entry copies a separate mirror without changing
    ///5533/5536. It must never alias the last41DD10 autosave mirror.
    pub(crate) fn freeze_rollback(&mut self) -> Result<CapturedNativeBody> {
        self.validate()?;
        let gaps = self.gaps.clone();
        let body = self.copy_native_sprite_records()?;
        Ok(CapturedNativeBody::capture(body, gaps))
    }
}
fn put_object_word(record: &mut [u8], object_offset: usize, value: u32) {
    record[object_offset - 8..object_offset - 4].copy_from_slice(&value.to_le_bytes());
}
fn put_object_float(record: &mut [u8], object_offset: usize, value: f32) {
    put_object_word(record, object_offset, value.to_bits());
}
fn object_float(record: &[u8], object_offset: usize) -> f32 {
    f32::from_bits(u32::from_le_bytes(
        record[object_offset - 8..object_offset - 4]
            .try_into()
            .unwrap(),
    ))
}
fn object_word(record: &[u8], object_offset: usize) -> u32 {
    u32::from_le_bytes(
        record[object_offset - 8..object_offset - 4]
            .try_into()
            .unwrap(),
    )
}
fn fresh_sprite() -> Vec<u8> {
    let mut record = vec![0; SPRITE_BYTES];
    for at in [112, 120, 116, 40, 36, 32] {
        put_object_float(&mut record, at, 1.);
    }
    for at in [132, 128, 736] {
        put_object_float(&mut record, at, 100.);
    }
    put_object_float(&mut record, 160, 1280.);
    put_object_float(&mut record, 164, 720.);
    put_object_word(&mut record, 184, 1);
    put_object_word(&mut record, 904, 50);
    //444180 writes integer clipping coordinates, not float bit patterns.
    for (at, value) in [(80, 0), (84, 0), (88, 1280), (92, 720)] {
        put_object_word(&mut record, at, value);
    }
    record
}
/// Legacy static-subset validator, retained for callers explicitly requesting
/// that subset. The live restore path uses native_control::from_record and
/// shared sequences; it does not call this integer/isotropic compatibility gate.
pub(crate) fn validate_static_sprite(record: &[u8]) -> Result<()> {
    ensure!(
        record.len() == SPRITE_BYTES,
        "Circus restored sprite extent"
    );
    let original = fresh_sprite();
    for (at, (actual, default)) in record.iter().zip(&original).enumerate() {
        let object = at + 8;
        let projected = (8..16).contains(&object)
            || (56..80).contains(&object)
            || (96..112).contains(&object)
            || (128..136).contains(&object)
            || (152..160).contains(&object)
            || (168..192).contains(&object)
            || (904..908).contains(&object);
        ensure!(
            actual == default || projected,
            "Circus restored sprite field+{object} needs native projection"
        );
    }
    for object in [
        8, 12, 64, 68, 72, 76, 96, 100, 104, 108, 128, 132, 152, 156, 168, 172, 176, 180,
    ] {
        ensure!(
            object_float(record, object).is_finite(),
            "Circus restored nonfinite sprite scalar"
        );
    }
    ensure!(
        object_word(record, 904) <= 100,
        "Circus restored sprite painter range"
    );
    for object in [8, 12, 128, 132, 152, 156] {
        let value = object_float(record, object);
        ensure!(
            value.fract() == 0. && value >= i16::MIN as f32 && value <= i16::MAX as f32,
            "Circus restored fractional/extended scalar requires retained object-state reconstruction"
        );
    }
    ensure!(
        object_float(record, 128) == object_float(record, 132),
        "Circus restored anisotropic scale requires retained object-state reconstruction"
    );
    Ok(())
}
pub(crate) fn sprite_for_slot(slot: u8) -> Option<usize> {
    Some(match slot {
        1 => 0,
        2 => 1,
        3 => 2,
        4 => 10,
        5 => 20,
        6 => 30,
        7 => 40,
        8 => 50,
        9 => 12,
        10 => 22,
        11 => 32,
        12 => 42,
        13 => 52,
        25 => 63,
        26 => 65,
        31 => 75,
        32 => 76,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn scene_bank_history_saturates_without_losing_the_comparison_bank() {
        let mut live = NativeLiveState::fresh();
        live.system_assignment(241, 0x12345678);
        for index in 0..1100 {
            live.body[118892 - LIVE_BASE + index] = 1;
        }
        live.record_scene_bank_changes().unwrap();
        assert_eq!(live.read32(115884), 1000);
        assert_eq!(live.read16(115888), 0);
        assert_eq!(live.read16(117886), 999);
        assert!(
            live.body[117888 - LIVE_BASE..118888 - LIVE_BASE]
                .iter()
                .all(|b| *b == 1)
        );
        assert!(
            live.body[119992 - LIVE_BASE..121092 - LIVE_BASE]
                .iter()
                .all(|b| *b == 1)
        );
        assert_eq!(live.read32(118888), 0x12345678);
        live.byte_assignment(10, 99, 9);
        live.record_scene_bank_changes().unwrap();
        assert_eq!(live.read32(115884), 1000);
        assert_eq!(live.body[121091 - LIVE_BASE], 9);
        // A later native reset clears the count, retaining raw history tails
        // and comparison bytes. Only subsequently changed entries are added.
        live.reset_script(false, &["A".into(), "B".into()]).unwrap();
        live.byte_assignment(0, 2, 77);
        live.byte_assignment(10, 99, 3);
        live.record_scene_bank_changes().unwrap();
        assert_eq!(live.read32(115884), 2);
        assert_eq!([live.read16(115888), live.read16(115890)], [2, 1099]);
        assert_eq!(&live.body[117888 - LIVE_BASE..117890 - LIVE_BASE], &[77, 3]);
    }
    #[test]
    fn completed_falling_policy_keeps_original_intensity_when_disabled() {
        let mut live = NativeLiveState::fresh();
        live.falling_effect(1, 43).unwrap();
        assert_eq!(live.read32(66337 * 4), 1);
        assert_eq!(live.read32(66338 * 4), 43);
        // Native425B90 clears the other effect families too, but preserves
        // the last falling intensity rather than zeroing the entire range.
        for i in [66336, 66339, 66340, 66341, 66342] {
            live.write32(i * 4, 9);
        }
        live.falling_effect(0, 0).unwrap();
        for i in [66336, 66337, 66339, 66340, 66341, 66342] {
            assert_eq!(live.read32(i * 4), 0);
        }
        assert_eq!(live.read32(66338 * 4), 43);
        let before = live.body.clone();
        assert!(live.falling_effect(7, 1).is_err());
        assert_eq!(live.body, before);
    }
    #[test]
    fn original_version_append_uses_encoded_bytes_and_retains_rejected_suffix() {
        let mut live = NativeLiveState::fresh();
        assert!(live.append_version(&"あ".repeat(61)).unwrap());
        assert!(live.append_version("12345").unwrap());
        let before = live.body.clone();
        assert!(!live.append_version("x").unwrap());
        assert!(!live.append_version("あ").unwrap());
        assert_eq!(live.body, before);
        assert!(live.append_version("\0").is_err());
        assert!(live.append_version("😀").is_err());
        assert_eq!(live.body, before);
        assert_eq!(live.body[265975 - LIVE_BASE], 0);
    }
    #[test]
    fn new_slot_tables_match_original_music_placement_and_voice_memory() {
        let mut live = NativeLiveState::fresh();
        live.music(8, "BGM\\M008").unwrap();
        assert_eq!(live.read32(21976), 8);
        let old = live.body[21945 - LIVE_BASE..21976 - LIVE_BASE].to_vec();
        live.music(255, "").unwrap();
        assert_eq!(live.read32(21976), 255);
        assert_eq!(live.body[21944 - LIVE_BASE], 0);
        assert_eq!(&live.body[21945 - LIVE_BASE..21976 - LIVE_BASE], &old);
        live.placement(19, "CHAR_A", [-20, 300]).unwrap();
        live.placement(19, "char_a", [12, -40]).unwrap();
        assert_eq!(live.read32(106156 + 19 * 4), 1);
        assert_eq!(live.read32(26152 + 19 * 4000 + 32), 12);
        assert_eq!(live.read32(26152 + 19 * 4000 + 36), (-40i32) as u32);
        live.voice_timing(101, "hero_a").unwrap();
        live.voice_timing(10, "HERO").unwrap();
        assert_eq!(live.read32(269844), 1);
        assert_eq!(live.read32(268008), (-15500i32) as u32);
        assert_eq!(
            &live.body[268012 - LIVE_BASE..268019 - LIVE_BASE],
            b"HERO_A\0"
        );
        live.voice_timing(255, "").unwrap();
        assert_eq!(live.read32(269844), 0);
        assert_eq!(live.read32(269848), 1);
        assert!(
            live.body[268008 - LIVE_BASE..269844 - LIVE_BASE]
                .iter()
                .all(|b| *b == 0)
        );
    }
    #[test]
    fn new_native_metadata_snapshot_roundtrip_rejects_wrong_extents() {
        let mut live = NativeLiveState::fresh();
        live.position(4, [1, 2]);
        live.gap("specific native operation pending");
        let encoded = serde_json::to_vec(&live).unwrap();
        let mut restored: NativeLiveState = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(restored.gaps(), live.gaps());
        assert!(restored.admitted_checkpoint(4).is_err());
        assert_eq!(
            restored
                .checkpoint(4)
                .unwrap()
                .sprite(10)
                .unwrap()
                .position(),
            [1., 2.]
        );
        let mut corrupt = serde_json::to_value(&live).unwrap();
        corrupt["body"] = serde_json::json!([1, 2]);
        assert!(serde_json::from_value::<NativeLiveState>(corrupt).is_err());
    }
    #[test]
    fn fresh_native_records_contain_original_nonzero_defaults_and_integer_clip() {
        let mut live = NativeLiveState::fresh();
        let image = live.admitted_checkpoint(0x123).unwrap();
        let sprite = image.sprite(149).unwrap();
        assert_eq!(sprite.position(), [0., 0.]);
        assert_eq!(sprite.scale_percent(), [100., 100.]);
        assert_eq!(sprite.pivot(), [0., 0.]);
        for (at, value) in [(80, 0), (84, 0), (88, 1280), (92, 720), (184, 1), (904, 50)] {
            assert_eq!(
                u32::from_le_bytes(sprite.bytes()[at - 8..at - 4].try_into().unwrap()),
                value
            );
        }
        assert_eq!(image.pc_fields().decode_next, 0x123);
        assert_eq!(image.pc_fields().checkpoint_resume, 0x123);
        assert!(image.sprite(11).unwrap().bytes().iter().all(|b| *b == 0));
    }
    #[test]
    fn byte_banks_reset_both_arrays_and_names_follow_original_copy_order() {
        let mut live = NativeLiveState::fresh();
        live.body[118892 - LIVE_BASE..121092 - LIVE_BASE].fill(77);
        live.reset_script(true, &["常坂".into(), "一登".into()])
            .unwrap();
        assert!(
            live.body[118892 - LIVE_BASE..121092 - LIVE_BASE]
                .iter()
                .all(|b| *b == 0)
        );
        assert_eq!(
            &live.body[265268 - LIVE_BASE..265273 - LIVE_BASE],
            b"\x88\xea\x93o\0"
        );
        assert_eq!(
            &live.body[265288 - LIVE_BASE..265293 - LIVE_BASE],
            b"\x8f\xed\x8d\xe2\0"
        );
        assert_eq!(live.read16(106240), u16::MAX);
        live.byte_assignment(10, 99, 255);
        assert_eq!(live.body[119991 - LIVE_BASE], 255);
        live.reset_script(false, &["常坂".into(), "一登".into()])
            .unwrap();
        assert_eq!(live.body[119991 - LIVE_BASE], 255);
    }
    #[test]
    fn read_history_metadata_contiguous_extension_and_tail_retention_are_native() {
        let mut live = NativeLiveState::fresh();
        live.reset_script(true, &["常坂".into(), "一登".into()])
            .unwrap();
        live.write32(111860, 7);
        live.write32(111868, 9);
        live.write32(111872, 11);
        live.write32(115884, 0x1234);
        live.read_label(3, 5, 17, 80, true).unwrap();
        live.system_assignment(233, 1);
        live.read_label(3, 6, 18, 90, true).unwrap();
        let body = NativeSaveBody::from_exact(&live.body).unwrap();
        assert_eq!(body.read_run_index(), 0);
        assert_eq!(
            body.read_run(0).unwrap(),
            NativeReadRun {
                scene_index: 3,
                metadata_and_flags: 0x9234,
                first_label: 5,
                last_label: 6,
                source_word_55936: 11,
                source_word_55930: 7,
                source_word_55934: 9
            }
        );
        live.read_label(4, 2, 19, 100, true).unwrap();
        let body = NativeSaveBody::from_exact(&live.body).unwrap();
        assert_eq!(body.read_run_index(), 1);
        let retained = body.read_run(1).unwrap();
        live.reset_script(false, &["常坂".into(), "一登".into()])
            .unwrap();
        assert_eq!(
            NativeSaveBody::from_exact(&live.body)
                .unwrap()
                .read_run(1)
                .unwrap(),
            retained
        );
    }
    #[test]
    fn transforms_texture_extents_and_random_history_are_synchronized_without_host_ids() {
        let mut live = NativeLiveState::fresh();
        live.attach_image(4, [2048, 1024], [1280, 720]).unwrap();
        live.position(4, [-20, 30]);
        live.scale(4, 150, [100, 200]);
        live.random_result(17);
        let body = live.admitted_checkpoint(9).unwrap();
        let sprite = body.sprite(10).unwrap();
        assert_eq!(sprite.position(), [-20., 30.]);
        assert_eq!(sprite.scale_percent(), [150., 150.]);
        assert_eq!(sprite.pivot(), [100., 200.]);
        assert_eq!(sprite.transient_texture_identity(), 0);
        assert_eq!(object_float(sprite.bytes(), 64), 2048.);
        assert_eq!(object_float(sprite.bytes(), 104), 1280.);
        assert_eq!(live.read32(111864), 1);
        assert_eq!(live.read32(115080), 17);
        live.audit_completed(0x49, "start", 10).unwrap();
        assert!(live.admitted_checkpoint(11).is_err());
        assert!(
            live.gaps()
                .contains("opcode0x49 live-body writes not yet audited")
        );
    }
    #[test]
    fn frozen_mirror_retains_capture_time_coverage_and_original_scene_marker() {
        let mut live = NativeLiveState::fresh();
        live.scene_marker(-1);
        live.audit_completed(0x9e, "start", 3).unwrap();
        let captured = live.freeze_checkpoint(9).unwrap();
        assert!(captured.projection_gaps().is_empty());
        assert_eq!(
            captured
                .admitted_body()
                .unwrap()
                .native_word(66508 * 4)
                .unwrap(),
            u32::MAX
        );
        live.scene_marker(7);
        live.audit_completed(0x49, "start", 10).unwrap();
        assert_eq!(
            captured
                .admitted_body()
                .unwrap()
                .native_word(66508 * 4)
                .unwrap(),
            u32::MAX
        );
        let blocked = live.freeze_checkpoint(11).unwrap();
        assert!(blocked.admitted_body().is_err());
        live.gaps.clear();
        assert!(
            blocked.admitted_body().is_err(),
            "later coverage cannot admit an older incomplete capture"
        );
    }
    #[test]
    fn rollback_mirror_is_separate_and_does_not_rewrite_decode_or_checkpoint_pc() {
        let mut live = NativeLiveState::fresh();
        let save = live.freeze_checkpoint(0x831).unwrap();
        live.write32(22132, 0x900);
        live.scene_marker(77);
        let rollback = live.freeze_rollback().unwrap();
        assert_eq!(
            rollback.admitted_body().unwrap().pc_fields().decode_next,
            0x900
        );
        assert_eq!(
            rollback
                .admitted_body()
                .unwrap()
                .pc_fields()
                .checkpoint_resume,
            0x831
        );
        assert_eq!(save.admitted_body().unwrap().pc_fields().decode_next, 0x831);
        assert_eq!(
            save.admitted_body()
                .unwrap()
                .native_word(66508 * 4)
                .unwrap(),
            0
        );
        assert_eq!(
            rollback
                .admitted_body()
                .unwrap()
                .native_word(66508 * 4)
                .unwrap(),
            77
        );
    }
    #[test]
    fn source_projection_receipts_are_single_use_and_keep_prior_gaps() {
        let mut live = NativeLiveState::fresh();
        live.stage_image(4, "BST\\HERO").unwrap();
        live.audit_completed(0x3c, "scene", 3).unwrap();
        assert!(live.gaps.is_empty());
        live.audit_completed(0x3c, "scene", 4).unwrap();
        assert!(
            live.gaps
                .contains("opcode0x3c live-body writes not yet audited")
        );
        live.stage_image(4, "BST\\HERO").unwrap();
        live.audit_completed(0x3c, "scene", 5).unwrap();
        assert!(
            live.gaps
                .contains("opcode0x3c live-body writes not yet audited")
        );
        let mut immediate = NativeLiveState::fresh();
        immediate.stage_image(19, "BG\\WHITE").unwrap();
        immediate.audit_completed(0x3c, "scene", 3).unwrap();
        assert!(!immediate.gaps.is_empty());
    }
    #[test]
    fn stage_reset_preserves_filename_tail_and_native_detach_fields() {
        let mut live = NativeLiveState::fresh();
        live.image_name(4, "BST\\HERO").unwrap();
        live.attach_image(4, [2048, 1024], [1280, 720]).unwrap();
        let name = 19496 - LIVE_BASE + 4 * 48;
        let tail = live.body[name + 1..name + 48].to_vec();
        live.position(4, [20, 30]);
        put_object_float(&mut live.sprites[3], 152, 9.);
        put_object_word(&mut live.sprites[3], 960, 7);
        live.reset_stage(true).unwrap();
        assert_eq!(live.body[name], 0);
        assert_eq!(&live.body[name + 1..name + 48], tail);
        assert_eq!(object_float(&live.sprites[10], 8), 20.);
        assert_eq!(object_float(&live.sprites[10], 64), 0.);
        assert_eq!(object_float(&live.sprites[3], 152), 0.);
        assert_eq!(object_word(&live.sprites[3], 960), 0);
        live.audit_completed(0x49, "scene", 3).unwrap();
        assert!(live.gaps.is_empty());
    }
    #[test]
    fn discard_scene_history_restores_native_cursors_and_retains_popped_bytes() {
        let mut live = NativeLiveState::fresh();
        let mut body = NativeSaveBody::from_exact(&live.body).unwrap();
        for (n, scene, x, y) in [(0, 2, 1, 2), (1, 3, 11, 12), (2, 3, 21, 22)] {
            body.set_read_run(
                n,
                NativeReadRun {
                    scene_index: scene,
                    metadata_and_flags: 0,
                    first_label: 1,
                    last_label: 2,
                    source_word_55936: 0,
                    source_word_55930: x,
                    source_word_55934: y,
                },
            )
            .unwrap();
        }
        body.set_read_run_index(2).unwrap();
        live.body.copy_from_slice(body.bytes());
        live.write32(27969 * 4, 71);
        live.discard_scene_read_history(3).unwrap();
        let after = NativeSaveBody::from_exact(&live.body).unwrap();
        assert_eq!(after.read_run_index(), 0);
        assert_eq!(after.read_run(1).unwrap(), body.read_run(1).unwrap());
        assert_eq!(after.read_run(2).unwrap(), body.read_run(2).unwrap());
        assert_eq!(live.read32(27964 * 4), 11);
        assert_eq!(live.read32(27966 * 4), 12);
        assert_eq!(live.read32(27968 * 4), 71);
        live.discard_scene_read_history(2).unwrap();
        assert_eq!(
            NativeSaveBody::from_exact(&live.body)
                .unwrap()
                .read_run(0)
                .unwrap()
                .scene_index,
            u16::MAX
        );
        assert_eq!(live.read32(27964 * 4), 0);
        assert_eq!(live.read32(27966 * 4), 0);
    }
    #[test]
    fn one_native_resource_detaches_every_bound_sprite_without_changing_crop() {
        let mut live = NativeLiveState::fresh();
        live.load_image(22, "ATLAS").unwrap();
        for object in [96, 97] {
            live.bind_resource(
                22,
                object,
                Some([720, 0, 998, 256]),
                Some(([2048, 1024], [1280, 720])),
            )
            .unwrap();
        }
        live.unload_image(22).unwrap();
        for object in [96, 97] {
            let sprite = &live.sprites[object];
            assert_eq!(object_float(sprite, 64), 0.);
            assert_eq!(object_float(sprite, 68), 0.);
            assert_eq!(object_word(sprite, 184), 0);
            assert_eq!(object_float(sprite, 96), 720.);
            assert_eq!(object_float(sprite, 104), 998.);
        }
    }
    #[test]
    fn detached_transfer_source_name_survives_later_unload_without_a_handle() {
        let mut live = NativeLiveState::fresh();
        live.load_image(19, "BG\\WHITE").unwrap();
        live.transfer_image(19, 1, false, Some(([2048, 1024], [1280, 720])))
            .unwrap();
        live.unload_image(19).unwrap();
        assert_eq!(
            live.checkpoint(3).unwrap().image_name_bytes(19).unwrap(),
            b"BG\\WHITE"
        );
    }
    #[test]
    fn movie_clear_obeys_pointer_protection_but_clears_both_special_effects() {
        let mut live = NativeLiveState::fresh();
        for slot in [1, 26, 27, 37] {
            live.load_image(slot, "IMAGE").unwrap();
        }
        live.write32(66336 * 4, 1);
        live.write32(66341 * 4, 1);
        live.write32(66510 * 4, 1);
        put_object_word(&mut live.sprites[10], 740, 1);
        put_object_word(&mut live.sprites[10], 908, 1);
        live.movie_screen_clear(true, true).unwrap();
        let saved = live.checkpoint(3).unwrap();
        assert_eq!(saved.image_name_bytes(1).unwrap(), b"IMAGE");
        assert_eq!(saved.image_name_bytes(37).unwrap(), b"IMAGE");
        assert!(saved.image_name_bytes(26).unwrap().is_empty());
        assert!(saved.image_name_bytes(27).unwrap().is_empty());
        assert_eq!(live.read32(66510 * 4), 0);
        assert_eq!(object_word(&live.sprites[10], 740), 0);
        assert_eq!(object_word(&live.sprites[10], 908), 0);
        live.movie_screen_clear(false, true).unwrap();
        assert!(
            live.checkpoint(3)
                .unwrap()
                .image_name_bytes(1)
                .unwrap()
                .is_empty()
        );
    }
    #[test]
    fn inline_pointer_handoff_keeps_original_source_filename() {
        let mut live = NativeLiveState::fresh();
        live.image_name(19, "BG\\WHITE").unwrap();
        live.transfer_image(19, 1, false, Some(([2048, 1024], [1280, 720])))
            .unwrap();
        let body = live.checkpoint(3).unwrap();
        assert_eq!(body.image_name_bytes(19).unwrap(), b"BG\\WHITE");
        assert_eq!(body.image_name_bytes(1).unwrap(), b"BG\\WHITE");
        assert_eq!(object_float(body.sprite(0).unwrap().bytes(), 64), 2048.);
    }
    #[test]
    fn backend2_playlist_shadow_copies_only_at_checkpoint_and_preserves_stop_tail() {
        let mut live = NativeLiveState::fresh();
        let movie = 266340 - LIVE_BASE;
        live.body[movie..movie + 1668].fill(77);
        live.movie_backend_selected(Some(2)).unwrap();
        for n in 0..20 {
            live.movie_playlist(&format!("movie{n}")).unwrap();
        }
        live.movie_playlist("abcdefghijklmnopqrstuvwxyz0123456789")
            .unwrap();
        assert!(live.body[movie..movie + 1668].iter().all(|v| *v == 77));
        let rollback = live.freeze_rollback().unwrap();
        assert!(
            rollback
                .admitted_body()
                .unwrap()
                .movie_record()
                .iter()
                .all(|v| *v == 77)
        );
        let body = live.checkpoint(3).unwrap();
        assert_eq!(&body.movie_record()[..7], b"movie0\0");
        assert_eq!(
            &body.movie_record()[1640..],
            b"abcdefghijklmnopqrstuvwxyz01"
        );
        live.movie_stopped();
        let body = live.checkpoint(4).unwrap();
        assert!(body.movie_record()[..1644].iter().all(|v| *v == 0));
        assert_eq!(&body.movie_record()[1644..], b"efghijklmnopqrstuvwxyz01");
    }
    #[test]
    fn audited_gui_heap_operations_do_not_waive_saved_sprite_or_syscall_mutations() {
        let mut live = NativeLiveState::fresh();
        for opcode in [
            0x0b, 0x15, 0x16, 0x17, 0x1a, 0x24, 0x32, 0x34, 0x35, 0x43, 0x68, 0x72, 0x73, 0x76,
            0x78, 0x7a, 0x7b, 0x87,
        ] {
            live.audit_completed(opcode, "scene", 3).unwrap();
        }
        assert!(live.admitted_checkpoint(4).is_ok());
        let mut settings = NativeLiveState::fresh();
        settings.syscall_completed(26);
        settings.audit_completed(0x18, "scene", 3).unwrap();
        assert!(settings.admitted_checkpoint(4).is_ok());
        let mut particles = NativeLiveState::fresh();
        particles.syscall_completed(18);
        particles.audit_completed(0x18, "scene", 3).unwrap();
        assert!(particles.admitted_checkpoint(4).is_err());
        let mut preview = NativeLiveState::fresh();
        preview.system_function_completed(203, 0);
        preview.audit_completed(0x1e, "scene", 3).unwrap();
        assert!(preview.admitted_checkpoint(4).is_err());
        let mut queued = NativeLiveState::fresh();
        queued.audit_completed(0x95, "scene", 3).unwrap();
        assert!(queued.admitted_checkpoint(4).is_err());
        let mut backend1 = NativeLiveState::fresh();
        backend1.movie_backend_selected(Some(1)).unwrap();
        backend1.audit_completed(0xab, "scene", 3).unwrap();
        assert!(backend1.admitted_checkpoint(4).is_err());
    }
}
