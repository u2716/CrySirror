//! Original IMOSAVE110 envelope metadata, not a substitute VM snapshot format.
//! Native 6B6AA4/6B6F78: checksum envelope, 138-byte header, eight UTF-16
//! strings, length-prefixed JPEG and encoded VM body. The metadata query never
//! decompresses or restores that body (native dat\g uses a5=0).
use super::*;
mod native;
use crysirror_core::{image_processing::ImageProcessState, service::ClientStorage};
use crysirror_resource::lane_feedback;

pub(super) const LIMIT: u64 = 64 * 1024 * 1024;
#[derive(Clone)]
pub(super) struct Metadata {
    date: f64,
    strings: [String; 8],
}
pub(super) enum ReadResult {
    Missing,
    Invalid(String),
    Ready(Metadata),
    Mutated(bool),
}

fn take<'a>(source: &mut &'a [u8], len: usize) -> Result<&'a [u8]> {
    ensure!(source.len() >= len, "truncated QLIE save envelope");
    let (head, rest) = source.split_at(len);
    *source = rest;
    Ok(head)
}
fn length(source: &mut &[u8]) -> Result<usize> {
    let n = i32::from_le_bytes(take(source, 4)?.try_into().unwrap());
    usize::try_from(n).context("negative QLIE save block length")
}
/// Complete original envelope. Body bytes retain their native codec and VM
/// layout; an engine checkpoint is never relabelled as an original save body.
pub(super) struct Envelope {
    pub metadata: Metadata,
    header: [u8; 138],
    pub thumbnail: Vec<u8>,
    encoded_body: Vec<u8>,
}
impl Envelope {
    pub(super) fn parse(mut bytes: &[u8], key: u32) -> Result<Self> {
        ensure!(
            take(&mut bytes, 4)? == 0xfd324d4eu32.to_le_bytes(),
            "QLIE save envelope tag"
        );
        let checksum = u32::from_le_bytes(take(&mut bytes, 4)?.try_into()?);
        let size = length(&mut bytes)?;
        ensure!(size as u64 <= LIMIT, "QLIE save payload budget");
        let mut body = take(&mut bytes, size)?;
        ensure!(bytes.is_empty(), "QLIE save envelope trailing bytes");
        ensure!(
            lane_feedback::checksum(body) == checksum,
            "QLIE save envelope checksum"
        );
        let header: [u8; 138] = take(&mut body, 138)?.try_into()?;
        ensure!(
            header[..16].split(|b| *b == 0).next() == Some(b"IMOSAVE110"),
            "QLIE save signature"
        );
        let mut tag: [u8; 40] = header[16..56].try_into()?;
        lane_feedback::decrypt(&mut tag, key);
        ensure!(
            tag[..32] == *b"we8wq8q986sfaifg8try695hj5iukt5w" && tag[32..].iter().all(|b| *b == 0),
            "QLIE save key verification"
        );
        let version = i32::from_le_bytes(header[56..60].try_into()?);
        ensure!(
            (0..=11).contains(&version),
            "unsupported QLIE save version {version}"
        );
        let date = f64::from_le_bytes(header[62..70].try_into()?);
        let mut strings = std::array::from_fn(|_| String::new());
        for string in &mut strings {
            let count = usize::try_from(i16::from_le_bytes(take(&mut body, 2)?.try_into()?))
                .context("negative QLIE save string length")?;
            let units = take(&mut body, count * 2)?
                .chunks_exact(2)
                .map(|p| u16::from_le_bytes([p[0], p[1]]))
                .collect::<Vec<_>>();
            *string = String::from_utf16(&units).context("QLIE save metadata UTF16")?;
        }
        let image_size = length(&mut body)?;
        let thumbnail = take(&mut body, image_size)?.to_vec();
        let data_size = length(&mut body)?;
        let encoded_body = take(&mut body, data_size)?.to_vec();
        ensure!(body.is_empty(), "QLIE save payload trailing bytes");
        Ok(Self {
            metadata: Metadata { date, strings },
            header,
            thumbnail,
            encoded_body,
        })
    }
    ///6B6AA4 decodes the original body only for a full load. Metadata and
    ///thumbnail operations keep its original bytes and avoid decompression.
    pub(super) fn decode_body(&self, key: u32) -> Result<Vec<u8>> {
        let mut bytes = self.encoded_body.clone();
        if self.header[60] != 0 {
            lane_feedback::decrypt(&mut bytes, key);
        }
        if self.header[61] != 0 {
            bytes = match u32::from_le_bytes(self.header[70..74].try_into()?) {
                0 => crysirror_resource::byte_pair::decode(&bytes, LIMIT as usize)?,
                1 => crysirror_resource::word_run_length::decode(&bytes, LIMIT as usize)?,
                method => bail!("unsupported QLIE save body compression {method}"),
            };
        }
        ensure!(bytes.len() as u64 <= LIMIT, "QLIE save decoded body budget");
        Ok(bytes)
    }
    /// Preserve every native header field and body byte while updating only
    ///the requested native metadata slots (tdat/dat\c policy).
    pub(super) fn encode(&self) -> Result<Vec<u8>> {
        let mut body = self.header.to_vec();
        for string in &self.metadata.strings {
            let units = string.encode_utf16().collect::<Vec<_>>();
            body.extend_from_slice(&i16::try_from(units.len())?.to_le_bytes());
            for unit in units {
                body.extend_from_slice(&unit.to_le_bytes());
            }
        }
        for block in [&self.thumbnail, &self.encoded_body] {
            body.extend_from_slice(&i32::try_from(block.len())?.to_le_bytes());
            body.extend_from_slice(block);
        }
        ensure!(
            body.len() as u64 <= LIMIT,
            "QLIE save encoded envelope budget"
        );
        let mut out = 0xfd324d4eu32.to_le_bytes().to_vec();
        out.extend_from_slice(&lane_feedback::checksum(&body).to_le_bytes());
        out.extend_from_slice(&i32::try_from(body.len())?.to_le_bytes());
        out.extend_from_slice(&body);
        Ok(out)
    }
    /// Replace an already serialized original VM body using the selected
    ///native stream profile. This never serializes CrySirror JSON as native.
    pub(super) fn set_body(&mut self, bytes: &[u8], key: u32, mode: u8) -> Result<()> {
        ensure!(
            bytes.len() as u64 <= LIMIT,
            "QLIE save original body budget"
        );
        let mut encoded = match mode {
            0 => bytes.to_vec(),
            1 => crysirror_resource::byte_pair::encode_literals(bytes, LIMIT as usize)?,
            2 => crysirror_resource::word_run_length::encode_literals(bytes, LIMIT as usize)?,
            _ => bail!("unsupported QLIE save encoding mode {mode}"),
        };
        self.header[60] = 1;
        self.header[61] = u8::from(mode != 0);
        self.header[70..74].copy_from_slice(&u32::from(mode == 2).to_le_bytes());
        lane_feedback::encrypt(&mut encoded, key);
        self.encoded_body = encoded;
        Ok(())
    }
}
impl Metadata {
    fn parse(bytes: &[u8], key: u32) -> Result<Self> {
        Ok(Envelope::parse(bytes, key)?.metadata)
    }
    fn calendar(&self) -> Result<[i32; 7]> {
        // Delphi TDateTime: signed days since1899-12-30; negative dates use
        // an absolute fractional time. Native yyyy/mm/dd/hh/nn/ss/zzz queries
        // read local civil fields, without applying the host's timezone.
        ensure!(self.date.is_finite(), "nonfinite QLIE save date");
        let days = self.date.trunc();
        ensure!(
            (-693593. ..=2958465.).contains(&days),
            "QLIE save calendar range"
        );
        let (year, month, day) =
            crysirror_core::service::normalize_civil_date(1899, 12, 30 + days as i32)
                .context("QLIE save calendar conversion")?;
        let ms = (self.date.fract().abs() * 86400000.).round_ties_even() as u64 % 86400000;
        Ok([
            year,
            i32::from(month),
            i32::from(day),
            (ms / 3600000) as i32,
            (ms / 60000 % 60) as i32,
            (ms / 1000 % 60) as i32,
            (ms % 1000) as i32,
        ])
    }
}
pub(super) fn read(
    storage: Arc<dyn ClientStorage>,
    name: &str,
    key: u32,
    cancel: &crysirror_tasks::Cancellation,
) -> Result<ReadResult> {
    cancel.check()?;
    let Some(info) = storage.metadata(Mount::Save, name)? else {
        return Ok(ReadResult::Missing);
    };
    ensure!(
        !info.is_directory && info.size.is_none_or(|size| size <= LIMIT),
        "QLIE original save byte budget/type"
    );
    let reader = storage.open_read(&VfsPath::new(Mount::Save, name)?)?;
    let data = crysirror_resource::read_all(reader.as_ref(), LIMIT)?;
    cancel.check()?;
    Ok(match Metadata::parse(&data, key) {
        Ok(meta) => ReadResult::Ready(meta),
        Err(error) => ReadResult::Invalid(format!("{error:#}")),
    })
}
#[derive(Clone)]
pub(super) enum Mutation {
    Delete,
    Copy { destination: String },
    SetMetadata { strings: Vec<String> },
}
fn mutate(
    storage: Arc<dyn ClientStorage>,
    name: &str,
    key: u32,
    operation: Mutation,
    cancel: &crysirror_tasks::Cancellation,
) -> Result<ReadResult> {
    cancel.check()?;
    let Some(info) = storage.metadata(Mount::Save, name)? else {
        return Ok(ReadResult::Mutated(false));
    };
    ensure!(
        !info.is_directory && info.size.is_none_or(|n| n <= LIMIT),
        "QLIE native save mutation budget/type"
    );
    let source = VfsPath::new(Mount::Save, name)?;
    if matches!(operation, Mutation::Delete) {
        storage.remove(&source)?;
        return Ok(ReadResult::Mutated(true));
    }
    let reader = storage.open_read(&source)?;
    let bytes = crysirror_resource::read_all(reader.as_ref(), LIMIT)?;
    cancel.check()?;
    let (destination, bytes) = match operation {
        Mutation::Copy { destination } => (VfsPath::new(Mount::Save, &destination)?, bytes),
        Mutation::SetMetadata { strings } => {
            let mut envelope = Envelope::parse(&bytes, key)?;
            for (slot, value) in strings.into_iter().take(8).enumerate() {
                envelope.metadata.strings[slot] = value;
            }
            (source, envelope.encode()?)
        }
        Mutation::Delete => unreachable!(),
    };
    cancel.check()?;
    storage.write_atomic(&destination, &bytes)?;
    Ok(ReadResult::Mutated(true))
}
impl QlieVm {
    fn native_save_key(&self) -> u32 {
        self.text
            .get("system/etc/game.txt")
            .into_iter()
            .flat_map(|s| s.lines())
            .filter_map(|line| line.split_once('='))
            .find(|(key, _)| key.trim().eq_ignore_ascii_case("CodeKey"))
            .and_then(|(_, value)| crate::expression::parse_integer(value))
            .unwrap_or(81789) as u32
            & 0xffffff
    }
    pub(super) fn mutate_save(
        &mut self,
        h: &mut dyn HostServices,
        m: &mut Machine,
        name: &str,
        operation: Mutation,
    ) -> Result<bool> {
        let name = crate::program::canonical(name)?;
        let completion = format!("mutation/{name}");
        if self.save_query.is_none() {
            let storage = h.storage(self.client)?;
            let key = self.native_save_key();
            let source = name.clone();
            self.save_query = Some((
                completion.clone(),
                self.pool
                    .spawn(move |cancel| mutate(storage, &source, key, operation, &cancel))?,
            ));
        }
        let (expected, task) = self.save_query.as_mut().unwrap();
        ensure!(
            *expected == completion,
            "QLIE save mutation completion owner"
        );
        let Some(result) = task.try_take()? else {
            return Ok(false);
        };
        self.save_query = None;
        let ReadResult::Mutated(success) = result else {
            bail!("QLIE save mutation/metadata completion mismatch");
        };
        m.result(0, Value::Int(i32::from(success)));
        Ok(true)
    }
    pub(super) fn query_save(
        &mut self,
        h: &mut dyn HostServices,
        m: &mut Machine,
        name: &str,
        metadata: bool,
    ) -> Result<bool> {
        let name = crate::program::canonical(name)?;
        if self.save_query.is_none() {
            let storage = h.storage(self.client)?;
            // Native5054F4 defaults CodeKey to81789, masks it to24 bits.
            let key = self
                .text
                .get("system/etc/game.txt")
                .into_iter()
                .flat_map(|s| s.lines())
                .filter_map(|line| line.split_once('='))
                .find(|(key, _)| key.trim().eq_ignore_ascii_case("CodeKey"))
                .and_then(|(_, value)| crate::expression::parse_integer(value))
                .unwrap_or(81789) as u32
                & 0xffffff;
            let source = name.clone();
            self.save_query = Some((
                name.clone(),
                self.pool
                    .spawn(move |cancel| read(storage, &source, key, &cancel))?,
            ));
        }
        let (expected, task) = self.save_query.as_mut().unwrap();
        ensure!(*expected == name, "QLIE save metadata completion owner");
        let Some(result) = task.try_take()? else {
            return Ok(false);
        };
        self.save_query = None;
        for n in 0..16 {
            m.result(n, Value::Int(0));
            m.result_text(n, String::new());
        }
        match result {
            ReadResult::Missing => (),
            ReadResult::Mutated(_) => bail!("QLIE save metadata/mutation completion mismatch"),
            ReadResult::Invalid(error) => {
                // A corrupt/incompatible slot is the native failed lookup,
                // rather than an acknowledged restore or a usable save.
                h.append_log(self.client, &format!("QLIE save metadata {name}: {error}"))?;
            }
            ReadResult::Ready(meta) if metadata => {
                for (n, value) in meta.calendar()?.into_iter().enumerate() {
                    m.result(n, Value::Int(value));
                }
                for (n, value) in meta.strings.into_iter().enumerate() {
                    m.result_text(n, value);
                }
            }
            ReadResult::Ready(_) => m.result(0, Value::Int(1)),
        }
        Ok(true)
    }
}

/// One in-flight reader save/load. The invoking native instruction stays
/// pending until its completion; its identity guards resumed calls.
pub(super) struct SaveJob {
    owner: String,
    phase: SavePhase,
}
enum SavePhase {
    Prepare(Box<Prepare>),
    Encode(Task<Vec<u8>>),
    Write(Task<()>),
    Read(Task<Option<Vec<u8>>>),
    Rebuild,
}
struct Prepare {
    body: Vec<u8>,
    mode: u8,
    clock: Option<Box<dyn crysirror_core::service::CivilTimeQuery>>,
    date: Option<f64>,
    want_thumbnail: bool,
    thumbnail_job: Option<crysirror_core::image_processing::ImageJob>,
    thumbnail: Option<Arc<dyn crysirror_core::image_processing::ImageResource>>,
}
pub(super) enum SaveTarget {
    File(String),
    Memory { store: String, name: String },
}
impl SaveTarget {
    fn owner(&self, op: &str) -> String {
        match self {
            Self::File(name) => format!("file/{op}/{name}"),
            Self::Memory { store, name } => format!("{store}/{op}/{name}"),
        }
    }
}
impl Envelope {
    /// 6B6F78 header: IMOSAVE110 signature, encrypted key tag, version 11,
    /// cipher flag, compression flag/method and the Delphi TDateTime.
    pub(super) fn create(
        strings: [String; 8],
        date: f64,
        thumbnail: Vec<u8>,
        body: &[u8],
        key: u32,
        mode: u8,
    ) -> Result<Self> {
        let mut header = [0u8; 138];
        header[..10].copy_from_slice(b"IMOSAVE110");
        let mut tag = [0u8; 40];
        tag[..32].copy_from_slice(b"we8wq8q986sfaifg8try695hj5iukt5w");
        lane_feedback::encrypt(&mut tag, key);
        header[16..56].copy_from_slice(&tag);
        header[56..60].copy_from_slice(&11i32.to_le_bytes());
        header[62..70].copy_from_slice(&date.to_le_bytes());
        let mut envelope = Self {
            metadata: Metadata { date, strings },
            header,
            thumbnail,
            encoded_body: Vec::new(),
        };
        envelope.set_body(body, key, mode)?;
        Ok(envelope)
    }
}
/// Delphi TDateTime (Sysutils::Now): local days since 1899-12-30.
fn delphi_datetime(clock: &crysirror_core::service::CivilTime) -> f64 {
    let local = clock.unix_ms as f64 + f64::from(clock.utc_offset_seconds) * 1000.;
    local / 86400000. + 25569.
}
fn read_save(
    storage: Arc<dyn ClientStorage>,
    name: &str,
    cancel: &crysirror_tasks::Cancellation,
) -> Result<Option<Vec<u8>>> {
    cancel.check()?;
    let Some(info) = storage.metadata(Mount::Save, name)? else {
        return Ok(None);
    };
    ensure!(
        !info.is_directory && info.size.is_none_or(|size| size <= LIMIT),
        "QLIE original save byte budget/type"
    );
    let reader = storage.open_read(&VfsPath::new(Mount::Save, name)?)?;
    Ok(Some(crysirror_resource::read_all(reader.as_ref(), LIMIT)?))
}
/// `$$$SaveImage,<slot>.bmp`: the JPEG thumbnail stored in a save envelope.
pub(super) fn read_thumbnail(
    storage: Arc<dyn ClientStorage>,
    name: &str,
    key: u32,
    cancel: &crysirror_tasks::Cancellation,
) -> Result<Option<Vec<u8>>> {
    let Some(bytes) = read_save(storage, name, cancel)? else {
        return Ok(None);
    };
    let envelope = Envelope::parse(&bytes, key)?;
    Ok((!envelope.thumbnail.is_empty()).then_some(envelope.thumbnail))
}
impl QlieVm {
    pub(super) fn save_key(&self) -> u32 {
        self.native_save_key()
    }
    /// 6D5218 (dat\s, mdat\s, tdat\s). Flags: 1 embeds mdat, 4 skips the
    /// SystemData write, 8 skips the thumbnail. Mode selects 6B6F78's body
    /// codec (0 none, 1 byte pair, 2 word run length; native default 2).
    pub(super) fn save_reader(
        &mut self,
        h: &mut dyn HostServices,
        m: &mut Machine,
        target: SaveTarget,
        flags: i32,
        mode: i32,
    ) -> Result<bool> {
        let owner = target.owner("s");
        if self.save_job.is_none() {
            ensure!((0..=2).contains(&mode), "unsupported QLIE save encoding mode {mode}");
            if flags & 4 == 0 {
                self.persist_system_data(h, m, true)?;
            }
            let body = self.capture_state(h, m, flags)?;
            self.save_job = Some(SaveJob {
                owner: owner.clone(),
                phase: SavePhase::Prepare(Box::new(Prepare {
                    body,
                    mode: mode as u8,
                    clock: Some(h.begin_civil_time(self.client)?),
                    date: None,
                    want_thumbnail: flags & 8 == 0,
                    thumbnail_job: None,
                    thumbnail: None,
                })),
            });
        }
        ensure!(
            self.save_job.as_ref().unwrap().owner == owner,
            "QLIE save completion owner"
        );
        if let SavePhase::Prepare(_) = &self.save_job.as_ref().unwrap().phase {
            if !self.poll_prepare(h)? {
                return Ok(false);
            }
        }
        let job = self.save_job.as_mut().unwrap();
        if let SavePhase::Encode(task) = &mut job.phase {
            let Some(envelope) = task.try_take()? else {
                return Ok(false);
            };
            match &target {
                SaveTarget::File(name) => {
                    let storage = h.storage(self.client)?;
                    let path = VfsPath::new(Mount::Save, name)?;
                    job.phase = SavePhase::Write(self.pool.spawn(move |cancel| {
                        cancel.check()?;
                        storage.write_atomic(&path, &envelope)?;
                        Ok(())
                    })?);
                }
                SaveTarget::Memory { store, name } => {
                    self.stacks
                        .entry(store.clone())
                        .or_default()
                        .put(super::save_state::StackEntry {
                            name: name.clone(),
                            envelope,
                        });
                    self.save_job = None;
                    m.result(0, Value::Int(1));
                    return Ok(true);
                }
            }
        }
        let job = self.save_job.as_mut().unwrap();
        let SavePhase::Write(task) = &mut job.phase else {
            bail!("QLIE save completion phase")
        };
        if task.try_take()?.is_none() {
            return Ok(false);
        }
        self.save_job = None;
        m.result(0, Value::Int(1));
        Ok(true)
    }
    fn poll_prepare(&mut self, h: &mut dyn HostServices) -> Result<bool> {
        use crysirror_core::service::CivilTimeState;
        let SavePhase::Prepare(prepare) = &mut self.save_job.as_mut().unwrap().phase else {
            unreachable!()
        };
        if let Some(clock) = prepare.clock.as_mut() {
            match clock.poll()? {
                CivilTimeState::Pending => return Ok(false),
                CivilTimeState::Ready(time) => {
                    prepare.date = Some(delphi_datetime(&time));
                    prepare.clock = None;
                }
                CivilTimeState::Failed(error) => bail!("QLIE save clock: {error}"),
            }
        }
        if prepare.want_thumbnail && prepare.thumbnail.is_none() {
            if prepare.thumbnail_job.is_none() {
                let priority = self.thumbnail_priority();
                let Some(job) = self.begin_thumbnail(h, priority)? else {
                    return Ok(false);
                };
                let SavePhase::Prepare(prepare) = &mut self.save_job.as_mut().unwrap().phase
                else {
                    unreachable!()
                };
                prepare.thumbnail_job = Some(job);
            }
            let SavePhase::Prepare(prepare) = &mut self.save_job.as_mut().unwrap().phase else {
                unreachable!()
            };
            match prepare.thumbnail_job.as_mut().unwrap().poll()? {
                ImageProcessState::Pending => return Ok(false),
                ImageProcessState::Failed(error) => {
                    self.end_thumbnail(h)?;
                    self.save_job = None;
                    bail!("QLIE save thumbnail capture: {error}")
                }
                ImageProcessState::Ready(image) => {
                    prepare.thumbnail = Some(image);
                    prepare.thumbnail_job = None;
                    self.end_thumbnail(h)?;
                }
            }
        }
        let strings = self.save_metadata_strings();
        let key = self.save_key();
        let size = self.thumbnail_size();
        let SavePhase::Prepare(prepare) = std::mem::replace(
            &mut self.save_job.as_mut().unwrap().phase,
            SavePhase::Rebuild,
        ) else {
            unreachable!()
        };
        let Prepare {
            body,
            mode,
            date,
            thumbnail,
            ..
        } = *prepare;
        let date = date.unwrap_or(0.);
        self.save_job.as_mut().unwrap().phase = SavePhase::Encode(self.pool.spawn(move |cancel| {
            cancel.check()?;
            // 6E22A0 -> 560720 reduces the capture to ThumWidth/ThumHeight;
            // 6B6F78 stores a pf24bit TJPEGImage at CompressionQuality 94.
            let jpeg = match thumbnail {
                Some(image) => {
                    let rgba = crysirror_image::box_downscale::box_downscale_rgba(
                        image.size(),
                        &image.rgba(),
                        size,
                    )?;
                    crysirror_image::encode_jpeg(
                        &crysirror_image::ImageSurface {
                            width: size[0],
                            height: size[1],
                            rgba,
                        },
                        94,
                    )?
                }
                None => Vec::new(),
            };
            cancel.check()?;
            Envelope::create(strings, date, jpeg, &body, key, mode)?.encode()
        })?);
        Ok(true)
    }
    /// 6D4D24 (dat\l, mdat\l, tdat\l). ResultInt[0]=1 once the reader and
    /// every restored object's resources are reinstated; 0 if absent/invalid.
    pub(super) fn load_reader(
        &mut self,
        h: &mut dyn HostServices,
        m: &mut Machine,
        target: SaveTarget,
    ) -> Result<bool> {
        let owner = target.owner("l");
        if self.save_job.is_none() {
            match &target {
                SaveTarget::File(name) => {
                    self.persist_system_data(h, m, true)?;
                    let storage = h.storage(self.client)?;
                    let name = name.clone();
                    self.save_job = Some(SaveJob {
                        owner: owner.clone(),
                        phase: SavePhase::Read(
                            self.pool.spawn(move |cancel| read_save(storage, &name, &cancel))?,
                        ),
                    });
                }
                SaveTarget::Memory { store, name } => {
                    let bytes = self
                        .stacks
                        .get(store)
                        .and_then(|s| s.get(name))
                        .map(|e| e.envelope.clone());
                    self.save_job = Some(SaveJob {
                        owner: owner.clone(),
                        phase: SavePhase::Rebuild,
                    });
                    if !self.install_save(h, m, bytes)? {
                        self.save_job = None;
                        m.result(0, Value::Int(0));
                        return Ok(true);
                    }
                }
            }
        }
        ensure!(
            self.save_job.as_ref().unwrap().owner == owner,
            "QLIE load completion owner"
        );
        if let SavePhase::Read(task) = &mut self.save_job.as_mut().unwrap().phase {
            let Some(bytes) = task.try_take()? else {
                return Ok(false);
            };
            self.save_job.as_mut().unwrap().phase = SavePhase::Rebuild;
            if !self.install_save(h, m, bytes)? {
                self.save_job = None;
                m.result(0, Value::Int(0));
                return Ok(true);
            }
        }
        if !self.poll_rebuild(h)? {
            return Ok(false);
        }
        self.save_job = None;
        m.result(0, Value::Int(1));
        Ok(true)
    }
    /// False reports the native failed load (ResultInt 0) with a log entry.
    fn install_save(
        &mut self,
        h: &mut dyn HostServices,
        m: &mut Machine,
        bytes: Option<Vec<u8>>,
    ) -> Result<bool> {
        let Some(bytes) = bytes else {
            return Ok(false);
        };
        let key = self.save_key();
        let body = match Envelope::parse(&bytes, key).and_then(|e| e.decode_body(key)) {
            Ok(body) => body,
            Err(error) => {
                h.append_log(self.client, &format!("QLIE save load: {error:#}"))?;
                return Ok(false);
            }
        };
        if !body.starts_with(super::save_state::BODY_TAG) {
            let message =
                "QLIE save load: original-engine 6D5218 save body import is not implemented";
            h.append_log(self.client, message)?;
            eprintln!("CrySirror: {message}");
            return Ok(false);
        }
        self.begin_restore(h, m, &body)?;
        Ok(true)
    }
    /// Stack-store metadata (6FD388 ?gd/c/cp) shares the file envelope codec.
    pub(super) fn memory_metadata(&self, store: &str, name: &str) -> Option<Metadata> {
        let key = self.save_key();
        self.stacks
            .get(store)?
            .get(name)
            .and_then(|e| Envelope::parse(&e.envelope, key).ok())
            .map(|e| e.metadata)
    }
    pub(super) fn memory_set_metadata(&mut self, store: &str, name: &str, strings: &[String]) -> Result<bool> {
        let key = self.save_key();
        let Some(entry) = self
            .stacks
            .get_mut(store)
            .and_then(|s| s.entries.iter_mut().find(|e| e.name == name))
        else {
            return Ok(false);
        };
        let mut envelope = Envelope::parse(&entry.envelope, key)?;
        for (slot, value) in strings.iter().take(8).enumerate() {
            envelope.metadata.strings[slot] = value.clone();
        }
        entry.envelope = envelope.encode()?;
        Ok(true)
    }
    pub(super) fn report_metadata(&self, m: &mut Machine, meta: Option<Metadata>) -> Result<()> {
        for n in 0..16 {
            m.result(n, Value::Int(0));
            m.result_text(n, String::new());
        }
        if let Some(meta) = meta {
            for (n, value) in meta.calendar()?.into_iter().enumerate() {
                m.result(n, Value::Int(value));
            }
            for (n, value) in meta.strings.into_iter().enumerate() {
                m.result_text(n, value);
            }
        }
        Ok(())
    }
}
