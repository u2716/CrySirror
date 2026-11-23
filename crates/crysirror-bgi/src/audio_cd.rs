//! Native MCI/TMSF policy using scoped virtual CD-DA resources and engine audio.
use super::*;
use crysirror_bgi_vm::HostMemoryWrite;
use crysirror_core::service::{AudioParameters, AudioRequest, EncodedAudio, PcmBus};
use crysirror_resource::{ArchiveFormat, DecodeContext};
const CHANNEL: u32 = 0x1fe;

fn number(stem: &str) -> Option<u8> {
    let lower = stem.trim().to_ascii_lowercase();
    let digits = ["track", "cdda", "audio", "cd"]
        .into_iter()
        .find_map(|prefix| {
            lower
                .strip_prefix(prefix)
                .map(|suffix| suffix.trim_start_matches(['_', '-', ' ']))
        })
        .unwrap_or(&lower);
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let track = digits.parse::<u16>().ok()?;
    (1..=255).contains(&track).then_some(track as u8)
}
fn track_name(name: &str, packed: bool) -> Option<u8> {
    let leaf = name.rsplit(['/', '\\']).next()?;
    let (stem, extension) = leaf.rsplit_once('.')?;
    if !matches!(extension.to_ascii_lowercase().as_str(), "ogg" | "wav") {
        return None;
    }
    if packed
        && !["track", "cdda", "audio", "cd"]
            .iter()
            .any(|prefix| stem.to_ascii_lowercase().starts_with(prefix))
    {
        return None;
    }
    number(stem)
}
fn payload(bytes: &[u8]) -> bool {
    bytes.starts_with(b"OggS")
        || bytes.starts_with(b"RIFF")
        || crysirror_audio::wave_box::WaveBoxInfo::parse(bytes).is_some()
}

impl BgiVm {
    pub(super) fn cd_audio_call(
        &mut self,
        host: &mut dyn HostServices,
        vm: &mut Machine,
        call: &NativeCall,
    ) -> Result<bool> {
        let mut result = 0;
        let mut writes = vec![];
        match call.id {
            0x80 => {
                if !self.audio.cd_open {
                    if self.audio.cd_scan.is_none() {
                        let storage = host.storage(self.client)?;
                        let config = self.config.clone();
                        let client = self.client;
                        self.audio.cd_scan = Some(self.pool.spawn(move |cancel| {
                            let mut tracks = BTreeMap::new();
                            let mut retained = 0usize;
                            let root = &config.root;
                            let roots = storage.list(Mount::Asset, root)?;
                            let mut directories = vec![root.clone()];
                            for name in &roots {
                                if name.eq_ignore_ascii_case("cd")
                                    || name.eq_ignore_ascii_case("cdda")
                                {
                                    directories.push(if root.is_empty() {
                                        name.clone()
                                    } else {
                                        format!("{root}/{name}")
                                    });
                                } else if name.eq_ignore_ascii_case("audio") {
                                    let audio = if root.is_empty() {
                                        name.clone()
                                    } else {
                                        format!("{root}/{name}")
                                    };
                                    for sub in storage.list(Mount::Asset, &audio)? {
                                        if sub.eq_ignore_ascii_case("cdda") {
                                            directories.push(format!("{audio}/{sub}"));
                                        }
                                    }
                                }
                            }
                            directories.sort();
                            directories.dedup();
                            for directory in directories {
                                for name in storage.list(Mount::Asset, &directory)? {
                                    cancel.check()?;
                                    let Some(track) = track_name(&name, directory == *root) else {
                                        continue;
                                    };
                                    if tracks.contains_key(&track) {
                                        continue;
                                    }
                                    let path = if directory.is_empty() {
                                        name
                                    } else {
                                        format!("{directory}/{name}")
                                    };
                                    let reader =
                                        storage.open_asset(&VfsPath::new(Mount::Asset, &path)?)?;
                                    let bytes = crysirror_resource::read_all(
                                        reader.as_ref(),
                                        64 * 1024 * 1024,
                                    )?;
                                    if !payload(&bytes) {
                                        continue;
                                    }
                                    retained = retained
                                        .checked_add(bytes.len())
                                        .context("CD audio byte overflow")?;
                                    ensure!(
                                        retained <= 256 * 1024 * 1024,
                                        "CD audio retained byte budget"
                                    );
                                    tracks.insert(track, (path, Arc::from(bytes)));
                                }
                            }
                            for name in roots
                                .into_iter()
                                .filter(|name| name.to_ascii_lowercase().ends_with(".arc"))
                            {
                                cancel.check()?;
                                let path = config.asset(&name)?;
                                let context =
                                    DecodeContext::new(client).with_cancellation(cancel.clone());
                                let source =
                                    storage.open_asset(&VfsPath::new(Mount::Asset, &path)?)?;
                                if crysirror_formats_arc::BgiArc
                                    .probe(source.as_ref())?
                                    .confidence
                                    == 0
                                {
                                    continue;
                                }
                                let archive =
                                    crysirror_formats_arc::BgiArc.open(source, &context)?;
                                for entry in archive.entries() {
                                    let Some(track) = track_name(&entry.path, true) else {
                                        continue;
                                    };
                                    if tracks.contains_key(&track) {
                                        continue;
                                    }
                                    cancel.check()?;
                                    let bytes = crysirror_resource::read_all(
                                        archive.open(entry.id, &context)?.as_ref(),
                                        64 * 1024 * 1024,
                                    )?;
                                    if !payload(&bytes) {
                                        continue;
                                    }
                                    retained = retained
                                        .checked_add(bytes.len())
                                        .context("CD audio byte overflow")?;
                                    ensure!(
                                        retained <= 256 * 1024 * 1024,
                                        "CD audio retained byte budget"
                                    );
                                    tracks.insert(track, (entry.path.clone(), Arc::from(bytes)));
                                }
                            }
                            Ok(tracks)
                        })?);
                    }
                    let Some(tracks) = self.audio.cd_scan.as_mut().unwrap().try_take()? else {
                        return Ok(false);
                    };
                    self.audio.cd_scan = None;
                    self.audio.cd_open = !tracks.is_empty();
                    self.audio.cd_mode = if self.audio.cd_open { 3 } else { 0 };
                    self.audio.cd_tracks = tracks;
                }
            }
            0x81 => {
                if self.audio.cd_open {
                    host.submit_audio(
                        self.client,
                        AudioRequest::Stop {
                            channel: CHANNEL,
                            fade_ms: 0.0,
                        },
                    )?;
                }
                self.audio.cd_open = false;
                self.audio.cd_mode = 0;
                self.audio.cd_track = None;
                self.audio.cd_tracks.clear();
            }
            0x84 => {
                if self.audio.cd_open {
                    let track = call.args[0] as u8;
                    self.audio.cd_track = Some(track);
                    self.audio.cd_notify = call.args[1] != 0;
                    if let Some((_, bytes)) = self.audio.cd_tracks.get(&track) {
                        let ticket = host.begin_audio(
                            self.client,
                            AudioRequest::Play {
                                channel: CHANNEL,
                                source: EncodedAudio {
                                    bytes: bytes.clone(),
                                    frame_limit: None,
                                },
                                intro: None,
                                parameters: AudioParameters {
                                    bus: PcmBus::Music,
                                    ..Default::default()
                                },
                            },
                        )?;
                        self.audio.cd_mode = 2;
                        self.audio.cd_ticket(ticket, 1);
                        return self.poll_audio_playback(host, vm, call);
                    }
                }
            }
            0x85 => {
                if self.audio.cd_open {
                    host.submit_audio(
                        self.client,
                        AudioRequest::Stop {
                            channel: CHANNEL,
                            fade_ms: 0.0,
                        },
                    )?;
                    self.audio.cd_mode = 3;
                }
            }
            0x86 => {
                if self.audio.cd_open {
                    result = 1;
                    if call.args[0] != 0 {
                        writes.push(HostMemoryWrite {
                            address: call.args[0] as u32,
                            bytes: self.audio.cd_mode.to_le_bytes().to_vec(),
                        });
                    }
                }
            }
            _ => unreachable!(),
        }
        Self::audio_results(vm, call, result, writes)?;
        self.calls += 1;
        Ok(true)
    }
}
