// SPDX-License-Identifier: MPL-2.0
//! CSP1 child records are decoded off the owner thread. The old scene stays
//! live until images, indexed clips and audio replacements have been admitted.
use super::*;
use crate::adapter::{ScriptSources, Session, sound, visual};
use crate::vm::{RestoredVm, save_slot::csp1};
use crysirror_core::{
    path::{Mount, VfsPath},
    service::{
        AudioReplacementJob, AudioReplacementState, AudioRequest, ClientStorage, EncodedAudio,
    },
};
use crysirror_resource::{DecodeContext, read_all};
use crysirror_tasks::Cancellation;

pub(in crate::adapter) struct Load {
    pub vm: RestoredVm,
    scene: Scene,
    bitmaps: Vec<bitmap::Prepared>,
    clips: Vec<(usize, movie::Clip, bool)>,
    starts: Vec<usize>,
    effects: sound::Effects,
    music: sound::Music,
    audio: Vec<AudioRequest>,
    audio_job: Option<AudioReplacementJob>,
    thumbnail: Option<crate::adapter::thumbnail::Bitmap>,
}

impl Load {
    pub(in crate::adapter) fn prepare(
        vm: RestoredVm,
        thumbnail: Option<Vec<u8>>,
        images: &ScriptSources,
        sounds: &ScriptSources,
        bgm: &ScriptSources,
        storage: &dyn ClientStorage,
        root: Option<&VfsPath>,
        client: ClientId,
        cancel: &Cancellation,
        random: u32,
        generation: usize,
        mut effects: sound::Effects,
        mut music: sound::Music,
    ) -> Result<Self> {
        let mut scene = Scene {
            random,
            ..Default::default()
        };
        let mut bitmaps = Vec::new();
        let mut clips = Vec::new();
        let mut seen = BTreeSet::new();
        let load_image = |name: &str, frame: u32| {
            let mut context = DecodeContext::new(client).with_cancellation(cancel.clone());
            context
                .parameters
                .insert("purple-software/pb2-frame".into(), frame.into());
            visual::prepare_image(images, name, &context)
        };
        csp1::visit(&vm.records, |tag, keys, bytes| {
            cancel.check()?;
            ensure!(
                seen.insert((tag, keys.to_vec())),
                "duplicate CSP1 record {tag:#x}"
            );
            let key = |index: usize| usize::from(keys[index]);
            let word = |at| checkpoint::word(bytes, at);
            match tag {
                0x200 => scene.color = word(0),
                0x201 => scene.level = word(0),
                0x202 => scene.background = word(0) != 0,
                0x203 => scene.exclude_vm_time = word(0) != 0,
                0x400 => scene.advs[key(0)] = Some(adv::Adv::from_record(bytes)?),
                0x480 | 0x481 => {
                    let slot = if tag == 0x480 {
                        BitmapSlot::Global(key(0))
                    } else {
                        BitmapSlot::Adv(key(0), key(1))
                    };
                    bitmaps.push(bitmap::Picture::from_record(
                        slot, bytes, client, load_image,
                    )?);
                }
                0x4c0 | 0x4c1 => {
                    let id = if tag == 0x4c0 {
                        SpriteId::Global(keys[0])
                    } else {
                        SpriteId::Part(keys[0] as u8, keys[1] as u8)
                    };
                    let sprite =
                        Sprite::from_record(id, bytes, &mut || msvc_rand(&mut scene.random))?;
                    scene.sprites.insert(id, sprite);
                }
                0x500 => {
                    scene.messages[key(0)] = Some(text::Window::from_record(
                        bytes,
                        "default".into(),
                        generation,
                    )?)
                }
                0x600 => scene.shake = Some(shake::Shake::from_record(bytes)?),
                0x610 => {
                    scene.transition = Some(Transition {
                        reveal: word(12),
                        color: word(16),
                        duration: word(20),
                        elapsed: word(24),
                    })
                }
                0x620 => {
                    let native_name = &bytes[16..bytes.len() - 1057];
                    let name = decode_name(native_name)?;
                    let image = load_image(&name, 0)?;
                    let request = rule::Request {
                        name,
                        native_name: native_name.to_vec(),
                        target: word(0).try_into()?,
                        duration: word(4),
                        edge: word(8),
                        reveal: word(12) != 0,
                        screen: None,
                    };
                    let mut rule = rule::Active::new(&request, image);
                    rule.elapsed = word(bytes.len() - 4);
                    scene.rule = Some(rule);
                }
                0x700 => {
                    let tail = bytes.len() - 12;
                    let native_name = &bytes[..tail - 1];
                    let name = decode_name(native_name)?;
                    let basename = VfsPath::new(Mount::Asset, &name)?;
                    ensure!(
                        basename.relative() == name && !name.contains('/'),
                        "invalid CSP1 movie basename"
                    );
                    let root = root.context("CSP1 movie root missing")?;
                    let path = VfsPath::new(Mount::Asset, &format!("{}/{name}", root.relative()))?;
                    let path =
                        crysirror_resource::capability_lookup::resolve_ascii_case_insensitive(
                            storage, &path,
                        )?
                        .with_context(|| format!("CSP1 movie missing: {name}"))?;
                    let mut clip =
                        movie::Clip::read(storage.open_asset(&path)?, native_name.to_vec())?;
                    clip.load_options = [word(tail), word(tail + 4)];
                    clips.push((key(0), clip, word(tail + 8) != 0));
                }
                0x301 => effects.restore_csp_table(bytes)?,
                0x300 => music.restore_csp_name(bytes)?,
                _ => {}
            }
            Ok(())
        })?;
        for tag in (0xff..=0x900).filter(|&tag| csp1::required(tag)) {
            ensure!(
                seen.contains(&(tag, Vec::new())),
                "missing CSP1 record {tag:#06x}"
            );
        }
        for a in scene.advs.iter().flatten() {
            ensure!(
                scene.messages[a.message].is_some(),
                "missing CSP1 ADV message owner"
            );
        }
        for (&key, _) in &scene.sprites {
            if let SpriteId::Part(i, _) = key {
                ensure!(
                    scene.advs[i as usize].is_some(),
                    "missing CSP1 sprite owner"
                );
            }
        }
        let mut audio = Vec::new();
        if effects.settings.enabled {
            let mut decoded = vec![false; 48];
            for item in effects.reopen_plan() {
                cancel.check()?;
                let Some(reader) = sounds.open_entry(&item.name)? else {
                    continue;
                };
                audio.push(AudioRequest::Play {
                    channel: sound::device_channel(item.slot),
                    source: EncodedAudio {
                        bytes: read_all(reader.as_ref(), 32 * 1024 * 1024)?.into(),
                        frame_limit: None,
                    },
                    intro: None,
                    parameters: item.parameters,
                });
                decoded[item.slot] = true;
            }
            effects.set_availability(true, decoded);
        }
        if music.settings.enabled {
            if let Some(request) = music.reopen_request() {
                let (first, second) = request.read_sources(bgm, cancel)?;
                audio.push(request.recipe(1, first, second, music.gain()));
            }
        }
        cancel.check()?;
        let thumbnail = thumbnail
            .map(|bytes| {
                crate::adapter::thumbnail::Thumbnail::prepare_cvns(
                    bytes,
                    &DecodeContext::new(client).with_cancellation(cancel.clone()),
                )
            })
            .transpose()?;
        Ok(Self {
            vm,
            thumbnail,
            scene,
            bitmaps,
            clips,
            starts: Vec::new(),
            effects,
            music,
            audio,
            audio_job: None,
        })
    }

    pub(in crate::adapter) fn stage(
        &mut self,
        live: &Scene,
        host: &mut dyn HostServices,
        client: ClientId,
        gain: f32,
    ) -> Result<()> {
        self.scene.movies = live.movies.detached();
        for window in self.scene.messages.iter_mut().flatten() {
            let name = if window.native_face.is_empty() {
                &live.font_name
            } else {
                &window.native_face
            };
            let font = crate::adapter::message::resolve_face(host, client, name)?
                .unwrap_or_else(|| "default".into());
            window.resolve_restored_font(font);
        }
        for image in self.bitmaps.drain(..) {
            self.scene.install_bitmap(host, client, image)?;
        }
        for (index, clip, started) in self.clips.drain(..) {
            self.scene.movies.load(host, client, index, clip, gain)?;
            if started {
                self.starts.push(index);
            }
        }
        if !self.audio.is_empty() {
            self.audio_job =
                Some(host.begin_audio_replacement(client, std::mem::take(&mut self.audio))?);
        }
        Ok(())
    }

    pub(in crate::adapter) fn ready(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        gain: f32,
    ) -> Result<bool> {
        if self.scene.movies.poll_staged(host, client)? {
            return Ok(false);
        }
        let replacement = if let Some(job) = &mut self.audio_job {
            ensure!(job.client() == client, "foreign CSP1 audio job");
            match job.poll()? {
                AudioReplacementState::Pending => return Ok(false),
                AudioReplacementState::Failed(e) => return Err(e.into()),
                AudioReplacementState::Ready(r) => {
                    ensure!(r.client() == client, "foreign CSP1 audio receipt");
                    Some(r)
                }
            }
        } else {
            None
        };
        for &index in &self.starts {
            self.scene.movies.start(host, client, index, gain)?;
        }
        let channels: Vec<_> = sound::MUSIC_CHANNELS
            .into_iter()
            .chain(5..9)
            .chain((0..52).map(sound::device_channel))
            .collect();
        host.replace_audio_channels(client, &channels, replacement.as_ref())?;
        Ok(true)
    }

    pub(in crate::adapter) fn discard(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
    ) -> Result<()> {
        self.scene.reset_scene(host, client)
    }

    pub(in crate::adapter) fn commit(
        mut self,
        session: &mut Session,
        canvas: [u32; 2],
        host: &mut dyn HostServices,
        client: ClientId,
    ) -> Result<()> {
        let windows = std::array::from_fn(|i| self.scene.messages[i].as_ref().map(|w| w.shown));
        session
            .cvns
            .swap_restored(&mut self.scene, canvas, host.now_ms() as u32);
        session.effects = self.effects;
        session.music = self.music;
        session.voices = sound::Voices::new(session.voices.settings);
        session.cvns_slot = Some((self.vm.header.clone(), std::mem::take(&mut self.vm.records)));
        session.main_checkpoint = None;
        if let Some(bytes) = self.thumbnail {
            session.thumbnail.restore_cvns(bytes);
        }
        session.vm.publish_cvns_vm(self.vm, windows);
        session.cvns_loaded_before = true;

        let (visible, enabled) = session.vm.menu_options();
        if visible {
            if let Some(menu) = session.vm.save_menu() {
                session.menu.publish(host, client, menu, enabled, &[])?;
            }
        } else {
            session.menu.hide(host, client)?;
        }
        self.scene.reset_scene(host, client)
    }
}

fn decode_name(bytes: &[u8]) -> Result<String> {
    let (s, invalid) = encoding_rs::SHIFT_JIS.decode_without_bom_handling(bytes);
    ensure!(!invalid, "invalid CP932 resource name in CSP1");
    Ok(s.into_owned())
}

impl Scene {
    pub(in crate::adapter) fn restore_random(&self) -> u32 {
        self.random
    }
    fn swap_restored(&mut self, other: &mut Self, canvas: [u32; 2], clock: u32) {
        macro_rules! swap {($($name:ident),*)=>{$(std::mem::swap(&mut self.$name,&mut other.$name);)*}}
        swap!(
            bitmaps,
            pictures,
            movies,
            sprites,
            messages,
            advs,
            transition,
            shake,
            rule,
            background,
            color,
            level,
            random,
            exclude_vm_time
        );
        self.settings[5] = 0;
        self.settings[17] = 0;
        self.reset_clock(self.fixed_clock, clock);
        self.primary.invalidate_all(canvas);
    }
}
