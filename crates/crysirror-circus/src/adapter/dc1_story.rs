//! Projection of the original D.C. scenario runtime (crate::dc1_story) onto
//! shared Layer/Text/Image/Audio services. The native 4030D0/4013F0 draw
//! order is reproduced as retained layers; no framebuffer is owned here.
use super::*;
use crate::dc1_story::{Event as StoryEvent, Glyph, Input as StoryInput, Step, Story, Visual};
use crysirror_core::service::{AudioParameters, AudioPreparePhase, EncodedAudio, PcmBus};

pub(super) const STAGE: u32 = ROOT + 0x1000;
pub(super) const BEFORE: u32 = ROOT + 0x2000;
const UI: u32 = ROOT + 0x3000;
const WINDOW: u32 = UI + 1;
const SIDE: u32 = UI + 2;
const BUTTONS: u32 = UI + 3;
const NAME: u32 = UI + 10;
const TEXT: u32 = UI + 11;
const CURSOR: u32 = UI + 12;
const CHOICES: u32 = UI + 20;
const PLACE: u32 = UI + 60;
const EYECATCH: u32 = ROOT + 0x4000;
const SYSTEM: u32 = ROOT + 0x6000;
/// 42A400: 1 dB per timer17 tick from -9 dB to -50 dB.
const MUSIC_FADE_MS: f32 = 41. * 11.;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub(super) enum Kind {
    Opaque,
    Key,
    /// 4286E0 stores blue - 255; the shared rule wipe consumes 255 - blue.
    Inverted,
}

pub(super) struct Channel {
    ticket: Option<u64>,
    pub(super) playing: bool,
}
pub(super) struct StoryHost {
    pub(super) story: Story,
    clock: u64,
    last_host: u64,
    scene: Option<(String, Task<Arc<advtxt::Program>>)>,
    voice: Option<Task<Option<Arc<[u8]>>>>,
    voice_queue: Option<String>,
    channels: [Channel; 3],
    music_queue: Option<(i32, bool, u64)>,
    attenuation: [i32; 3],
    wipe: Option<Visual>,
    wipe_started: bool,
    ascent: BTreeMap<u8, f32>,
    pub(super) unsupported: Option<&'static str>,
    created: bool,
    option_shown: bool,
    system_ids: BTreeSet<u32>,
}

impl StoryHost {
    pub(super) fn new(story: Story, now: u64) -> Self {
        Self {
            story,
            clock: now,
            last_host: now,
            scene: None,
            voice: None,
            voice_queue: None,
            channels: [
                Channel { ticket: None, playing: false },
                Channel { ticket: None, playing: false },
                Channel { ticket: None, playing: false },
            ],
            music_queue: None,
            attenuation: [0; 3],
            wipe: None,
            wipe_started: false,
            ascent: BTreeMap::new(),
            unsupported: None,
            created: false,
            option_shown: false,
            system_ids: BTreeSet::new(),
        }
    }
}

/// Absent native elements stay as hidden placeholders: removing an id that
/// was never published is an error in the shared layer system.
fn hidden(id: u32, parent: u32) -> Value {
    json!({"op":"put","layer":{"id":id,"parent":parent,"rect":[0,0,1,1],"visible":false,"paint":{"kind":"solid","color":[0,0,0,0]}}})
}
/// Layer order is resolved across groups, so each native pass gets its own
/// band: stage 100, window/text 200, eyecatch 400, system 500, option 600.
pub(super) fn band(layers: &mut [Value], base: i32) {
    for layer in layers {
        if let Some(order) = layer["layer"]["order"].as_array().cloned() {
            let major = order.first().and_then(Value::as_i64).unwrap_or(0) as i32;
            layer["layer"]["order"][0] = json!(base + major);
        }
    }
}
fn char_image(name: &str) -> String {
    format!("char/{name}.CRX")
}

impl Dc1Vm {
    fn story_images(&self, host: &StoryHost) -> Vec<(String, Kind)> {
        let tables = &host.story.tables;
        let mut out = vec![
            (self.data.as_ref().unwrap().frontend.bar.path.clone(), Kind::Key),
            (tables.key_wait.path.clone(), Kind::Key),
            (tables.map_name.path.clone(), Kind::Key),
            (tables.menu_chip.path.clone(), Kind::Key),
            (tables.dialog_chip.path.clone(), Kind::Key),
        ];
        let mut visual = |visual: &Visual| {
            if visual.map.is_some() {
                out.push((tables.timer_back.clone(), Kind::Opaque));
                out.push((tables.timer_chip.path.clone(), Kind::Key));
                return;
            }
            if let Some(background) = &visual.background {
                out.push((char_image(background), Kind::Opaque));
            }
            if let Some((overlay, _, _)) = &visual.overlay {
                out.push((char_image(overlay), Kind::Opaque));
            }
            for sprite in visual.sprites.iter().filter(|sprite| sprite.present) {
                out.push((char_image(sprite.image()), Kind::Opaque));
                out.push((char_image(&sprite.mask()), Kind::Opaque));
            }
            if let Some(effect) = &visual.effect {
                out.push((effect.clone(), Kind::Key));
            }
        };
        visual(&host.story.current_visual());
        if let Some(before) = &host.wipe {
            visual(before);
        }
        if let Some(heroine) = host.story.eyecatch {
            let (back, chip) = &tables.eyecatches[heroine];
            out.push((back.clone(), Kind::Opaque));
            out.push((chip.path.clone(), Kind::Key));
        }
        if host.wipe.is_some() {
            out.push((
                tables.grades[host.story.grade as usize].clone(),
                Kind::Inverted,
            ));
        }
        out
    }

    pub(super) fn story_frame(
        &mut self,
        host: &mut dyn HostServices,
        input: InputFrame,
    ) -> Result<VmYield> {
        let mut state = self.story.take().context("DC1 scenario owner missing")?;
        let result = self.story_frame_inner(host, input, &mut state);
        self.story = Some(state);
        result
    }

    fn story_frame_inner(
        &mut self,
        host: &mut dyn HostServices,
        input: InputFrame,
        state: &mut StoryHost,
    ) -> Result<VmYield> {
        if let Some(reason) = state.unsupported {
            bail!("{reason} is not ported");
        }
        let now = host.now_ms();
        self.poll_audio(host, state)?;
        //Shared resources are prepared before native time advances; the
        //original loads synchronously inside its frame.
        for (name, kind) in self.story_images(state) {
            if !self.image_kind(host, &name, kind)? {
                state.last_host = now;
                return Ok(VmYield::BudgetExhausted);
            }
        }
        if let Some((name, task)) = &mut state.scene {
            let Some(program) = task.try_take()? else {
                state.last_host = now;
                return Ok(VmYield::BudgetExhausted);
            };
            let name = name.clone();
            state.story.insert_program(&name, program);
            state.scene = None;
        }
        let pressed = input.buttons_pressed;
        use crysirror_core::service::buttons as b;
        //418F70: left/Enter -> BA9F58, Space -> BA9F48 (BCDB28 == 5),
        //right/Esc/Backspace -> BA9F70, wheel/PageUp/PageDown -> BA9F94/98.
        let mut native = StoryInput {
            x: input.pointer_x,
            y: input.pointer_y,
            click: pressed & (b::MOUSE_LEFT | b::ENTER) != 0,
            hide: pressed & b::SPACE != 0,
            cancel: pressed & (b::MOUSE_RIGHT | b::ESCAPE | b::BACKSPACE) != 0,
            wheel_up: pressed & (b::WHEEL_UP | b::PAGE_UP) != 0,
            wheel_down: pressed & (b::WHEEL_DOWN | b::PAGE_DOWN) != 0,
            skip: input.buttons_down & b::CTRL != 0,
            held: input.buttons_down & b::MOUSE_LEFT != 0,
        };
        let elapsed = now.saturating_sub(state.last_host).min(100);
        state.last_host = now;
        //408F70 runs once per 41B2C0 Sleep(1) loop; native timers compare ms.
        for _ in 0..elapsed.max(1) {
            state.clock += 1;
            state.story.now = state.clock;
            let playing = state.channels[2].playing && host.audio_playing(self.client, 2)?;
            state.story.set_voice_playing(playing);
            match state.story.frame(native)? {
                Step::Running => {}
                Step::NeedScene(name) => {
                    self.request_scene(state, name)?;
                    break;
                }
                Step::Unsupported(reason) => {
                    state.unsupported = Some(reason);
                    break;
                }
            }
            native.click = false;
            native.hide = false;
            native.cancel = false;
            native.wheel_up = false;
            native.wheel_down = false;
            if !state.story.events.is_empty() {
                let events = std::mem::take(&mut state.story.events);
                let stop = events.iter().any(|e| matches!(e, StoryEvent::WipeBegin(_)));
                for event in events {
                    self.story_event(host, state, event)?;
                }
                if stop {
                    break;
                }
            }
            //A new image dependency suspends native time until it is ready.
            if self
                .story_images(state)
                .iter()
                .any(|(name, kind)| !self.images.contains_key(&(name.clone(), *kind)))
            {
                break;
            }
        }
        if state.story.request.take() == Some(crate::dc1_story::Request::ReturnToTitle) {
            //BAB40C -> 414760 state24 -> 414F70: release the scenario.
            for channel in 0..3 {
                host.submit_audio(self.client, AudioRequest::Stop { channel, fade_ms: 0. })?;
            }
            let groups = [STAGE, UI, EYECATCH, SYSTEM];
            let layers = groups
                .iter()
                .map(|id| json!({"op":"set_visible","id":id,"visible":false}))
                .collect();
            self.submit(host, layers, vec![])?;
            if state.option_shown {
                self.option_layers_hide(host)?;
            }
            self.return_to_title = true;
            return Ok(VmYield::BudgetExhausted);
        }
        if let Some((id, looping, at)) = state.music_queue {
            if now >= at {
                state.music_queue = None;
                self.play_music(host, state, id, looping)?;
            }
        }
        if let Some(path) = state.voice_queue.clone() {
            self.start_voice(host, state, path)?;
        }
        //Publish only complete frames: new dependencies hold the previous one.
        for (name, kind) in self.story_images(state) {
            if !self.image_kind(host, &name, kind)? {
                return Ok(VmYield::BudgetExhausted);
            }
        }
        self.project(host, state)?;
        if let Some(reason) = state.unsupported {
            bail!("{reason} is not ported");
        }
        Ok(VmYield::BudgetExhausted)
    }

    fn request_scene(&mut self, state: &mut StoryHost, name: String) -> Result<()> {
        if state.scene.is_some() {
            return Ok(());
        }
        let storage = self.storage.clone().context("DC1 storage")?;
        let path = format!("{name}.MES");
        let pool = self.pool.as_ref().context("DC1 worker pool")?;
        let task = pool.spawn(move |cancel| {
            cancel.check()?;
            let file = VfsPath::new(Mount::Asset, &path)?;
            let bytes = crysirror_resource::read_all(
                storage.open_read(&file)?.as_ref(),
                crate::MAX_SCRIPT_BYTES as u64,
            )?;
            Ok(Arc::new(advtxt::Program::parse(&bytes)?))
        })?;
        state.scene = Some((name, task));
        Ok(())
    }

    fn story_event(
        &mut self,
        host: &mut dyn HostServices,
        state: &mut StoryHost,
        event: StoryEvent,
    ) -> Result<()> {
        match event {
            StoryEvent::Music { id, looping } => {
                if state.channels[0].playing {
                    //42A130 while playing: 42A400 fades, then starts 153A71C.
                    host.submit_audio(
                        self.client,
                        AudioRequest::Stop { channel: 0, fade_ms: MUSIC_FADE_MS },
                    )?;
                    state.channels[0].playing = false;
                    state.music_queue = Some((id, looping, host.now_ms() + MUSIC_FADE_MS as u64));
                } else {
                    self.play_music(host, state, id, looping)?;
                }
            }
            StoryEvent::MusicFade => {
                state.music_queue = None;
                if state.channels[0].playing {
                    host.submit_audio(
                        self.client,
                        AudioRequest::Stop { channel: 0, fade_ms: MUSIC_FADE_MS },
                    )?;
                    state.channels[0].playing = false;
                }
            }
            StoryEvent::MusicStopNow => {
                state.music_queue = None;
                host.submit_audio(self.client, AudioRequest::Stop { channel: 0, fade_ms: 0. })?;
                state.channels[0].playing = false;
            }
            StoryEvent::MusicAttenuation(value) => state.attenuation[0] = value,
            StoryEvent::SoundAttenuation(value) => state.attenuation[1] = value,
            StoryEvent::VoiceAttenuation(value) => state.attenuation[2] = value,
            StoryEvent::Sound { id, looping } => {
                let name = self
                    .data
                    .as_ref()
                    .unwrap()
                    .system
                    .sound
                    .get(&id)
                    .context("DC1 System.MES sound index")?
                    .replace('\\', "/");
                if !state.story.settings.sound.0 {
                    return Ok(());
                }
                let gain = state.story.settings.sound.1 - 1000 - state.attenuation[1];
                self.play_asset(host, state, 1, name, looping, gain)?;
            }
            StoryEvent::SoundStop => {
                host.submit_audio(self.client, AudioRequest::Stop { channel: 1, fade_ms: 0. })?;
                state.channels[1].playing = false;
            }
            StoryEvent::Voice(path) => {
                state.voice = None;
                state.voice_queue = Some(path);
            }
            StoryEvent::VoiceStop => {
                state.voice = None;
                state.voice_queue = None;
                host.submit_audio(self.client, AudioRequest::Stop { channel: 2, fade_ms: 0. })?;
                state.channels[2].playing = false;
            }
            StoryEvent::WipeBegin(before) => {
                state.wipe = Some(*before);
                state.wipe_started = false;
            }
            StoryEvent::WipeEnd => {
                if state.wipe_started {
                    self.submit(
                        host,
                        vec![
                            json!({"op":"end_crossfade","id":BEFORE}),
                            json!({"op":"remove","id":BEFORE}),
                        ],
                        vec![],
                    )?;
                }
                state.wipe = None;
                state.wipe_started = false;
            }
        }
        Ok(())
    }

    fn play_music(
        &mut self,
        host: &mut dyn HostServices,
        state: &mut StoryHost,
        id: i32,
        looping: bool,
    ) -> Result<()> {
        let music = self
            .data
            .as_ref()
            .unwrap()
            .system
            .music
            .get(&id)
            .context("DC1 System.MES music index")?
            .clone();
        //42A130: 200 * directory parameter + BA9E24 (-900) - BA9E30.
        if !state.story.settings.music.0 {
            return Ok(());
        }
        let gain = 200 * music.native_parameter + state.story.settings.music.1 - state.attenuation[0];
        self.play_asset(host, state, 0, music.name.replace('\\', "/"), looping, gain)
    }

    fn play_asset(
        &mut self,
        host: &mut dyn HostServices,
        state: &mut StoryHost,
        channel: u32,
        path: String,
        looping: bool,
        gain_db100: i32,
    ) -> Result<()> {
        if let Some(ticket) = state.channels[channel as usize].ticket.take() {
            host.release_audio(self.client, ticket)?;
        }
        let parameters = AudioParameters {
            gain: 10f32.powf(gain_db100.max(-10000) as f32 / 2000.),
            bus: if channel == 0 { PcmBus::Music } else { PcmBus::Sfx },
            looping,
            paused: true,
            ..Default::default()
        };
        let ticket = host.begin_audio(
            self.client,
            AudioRequest::PlayAsset {
                channel,
                source: crysirror_core::service::AudioAsset {
                    path: VfsPath::new(Mount::Asset, &path)?,
                    frame_limit: None,
                },
                intro: None,
                parameters,
            },
        )?;
        state.channels[channel as usize].ticket = Some(ticket);
        state.channels[channel as usize].playing = true;
        Ok(())
    }

    fn start_voice(
        &mut self,
        host: &mut dyn HostServices,
        state: &mut StoryHost,
        path: String,
    ) -> Result<()> {
        if state.voice.is_none() {
            //429B60: VOICE\ maps into voice.dat and .wav into its .pcm member.
            let member = path
                .strip_prefix("VOICE/")
                .or_else(|| path.strip_prefix("voice/"))
                .context("DC1 voice path outside VOICE")?;
            let member = match member.rsplit_once('.') {
                Some((stem, _)) => format!("{stem}.pcm"),
                None => format!("{member}.pcm"),
            };
            let archive = match &self.voice_archive {
                Some(archive) => archive.clone(),
                None => {
                    let storage = self.storage.clone().context("DC1 storage")?;
                    let source = storage.open_read(&VfsPath::new(Mount::Asset, "voice.dat")?)?;
                    let context = DecodeContext::new(self.client);
                    let archive: Arc<dyn Archive> = Arc::from(
                        crysirror_resource::circus_voice::CircusVoice52.open(source, &context)?,
                    );
                    self.voice_archive = Some(archive.clone());
                    archive
                }
            };
            let client = self.client;
            let pool = self.pool.as_ref().context("DC1 worker pool")?;
            let storage = self.storage.clone().context("DC1 storage")?;
            let loose = path.clone();
            state.voice = Some(pool.spawn(move |cancel| {
                cancel.check()?;
                //429A90 voice.dat lookup, then the loose VOICE .pcm; when both
                //are absent 42CD60 fails and 429B60 plays nothing.
                let key = member.to_ascii_lowercase();
                if let Some(entry) = archive
                    .entries()
                    .iter()
                    .find(|entry| entry.path.to_ascii_lowercase() == key)
                {
                    let reader = archive.open(entry.id, &DecodeContext::new(client))?;
                    return Ok(Some(Arc::from(crysirror_resource::read_all(reader.as_ref(), 64 << 20)?)));
                }
                let stem = loose.rsplit_once('.').map_or(loose.as_str(), |(stem, _)| stem);
                let file = VfsPath::new(Mount::Asset, &format!("{stem}.pcm"))?;
                match storage.open_read(&file) {
                    Ok(reader) => Ok(Some(Arc::from(crysirror_resource::read_all(reader.as_ref(), 64 << 20)?))),
                    Err(_) => Ok(None),
                }
            })?);
        }
        let Some(bytes) = state.voice.as_mut().unwrap().try_take()? else {
            return Ok(());
        };
        state.voice = None;
        state.voice_queue = None;
        let Some(bytes) = bytes else {
            state.channels[2].playing = false;
            return Ok(());
        };
        if let Some(ticket) = state.channels[2].ticket.take() {
            host.release_audio(self.client, ticket)?;
        }
        //42A690: BA9E2C (-300) + 153A4B8 - BA9E38.
        let gain = state.story.settings.voice.1 - state.attenuation[2];
        let ticket = host.begin_audio(
            self.client,
            AudioRequest::PlayResident {
                channel: 2,
                source: EncodedAudio { bytes, frame_limit: None },
                intro: None,
                parameters: AudioParameters {
                    gain: 10f32.powf(gain as f32 / 2000.),
                    bus: PcmBus::Voice,
                    paused: true,
                    ..Default::default()
                },
            },
        )?;
        state.channels[2].ticket = Some(ticket);
        state.channels[2].playing = true;
        Ok(())
    }

    fn poll_audio(&mut self, host: &mut dyn HostServices, state: &mut StoryHost) -> Result<()> {
        for channel in 0..3u32 {
            let Some(ticket) = state.channels[channel as usize].ticket else {
                continue;
            };
            let prepared = host.audio_preparation(self.client, ticket)?;
            match prepared.phase {
                AudioPreparePhase::Pending => {}
                AudioPreparePhase::Failed => bail!(
                    "DC1 audio preparation: {}",
                    prepared.error.unwrap_or_else(|| "unknown error".into())
                ),
                AudioPreparePhase::Ready => {
                    host.submit_audio(
                        self.client,
                        AudioRequest::Pause { channel, paused: false, fade_ms: 0. },
                    )?;
                    host.release_audio(self.client, ticket)?;
                    state.channels[channel as usize].ticket = None;
                }
            }
        }
        Ok(())
    }

    /// 42B8C0: TimerBack, the 403E10 date card at 24,24 (403F50), TimerChip
    /// frame 6, three 468EE8 rows and the hovered preview.
    fn map_layers(&self, map: &crate::dc1_story::MapScreen, tables: &crate::dc1_frontend::Story, base: u32) -> Result<Vec<Value>> {
        let bar = &self.data.as_ref().unwrap().frontend.bar;
        let mut parts = vec![self.picture(&tables.timer_back, false, base + 30, [0, 0], None, 1)?];
        let [month, day, weekday] = map.date;
        ensure!(
            (1..13).contains(&month) && (1..32).contains(&day) && (0..7).contains(&weekday),
            "DC1 403E10 date fields"
        );
        let mut card = vec![([0, 0], 17)];
        if month < 10 {
            card.push(([32, 8], month + 25));
        } else {
            card.push(([16, 8], 26));
            card.push(([32, 8], month + 15));
        }
        if day < 10 {
            card.push(([88, 8], day + 25));
        } else {
            card.push(([72, 8], day / 10 + 25));
            card.push(([88, 8], day % 10 + 25));
        }
        card.push(([136, 8], weekday + 18));
        for slot in 0..7usize {
            match card.get(slot) {
                Some((at, frame)) => parts.push(self.atlas_picture(bar, *frame as u32, base + 31 + slot as u32, [24 + at[0], 24 + at[1]], 2)?),
                None => parts.push(hidden(base + 31 + slot as u32, base)),
            }
        }
        let chip = &tables.timer_chip;
        parts.push(self.atlas_picture(chip, 6, base + 38, [328, 304], 3)?);
        for (i, rect) in tables.map_rects.iter().enumerate() {
            let frame = if map.hover == i as i32 { rect[5] } else { rect[4] };
            parts.push(self.atlas_picture(chip, frame as u32, base + 39 + i as u32, [rect[0], rect[1]], 4)?);
        }
        parts.push(match map.preview {
            0 => self.atlas_picture(chip, 8, base + 42, [392, 200], 5)?,
            1 => self.atlas_picture(chip, 9, base + 42, [544, 168], 5)?,
            2 => self.atlas_picture(chip, 7, base + 42, [288, 328], 5)?,
            _ => hidden(base + 42, base),
        });
        for part in &mut parts {
            part["layer"]["parent"] = json!(base);
        }
        band(&mut parts, 100);
        Ok(parts)
    }

    fn visual_layers(&self, visual: &Visual, base: u32, shake: [i32; 2], tables: &crate::dc1_frontend::Story) -> Result<Vec<Value>> {
        let mut layers = Vec::new();
        if let Some(map) = &visual.map {
            for id in [1, 2, 10, 11, 12, 13, 20] {
                layers.push(hidden(base + id, base));
            }
            layers.extend(self.map_layers(map, tables, base)?);
            return Ok(layers);
        }
        for id in 30..43 {
            layers.push(hidden(base + id, base));
        }
        let [ox, oy] = [visual.offset[0] + shake[0], visual.offset[1] + shake[1]];
        fn put(layers: &mut Vec<Value>, value: Result<Value>, base: u32) -> Result<()> {
            let mut value = value?;
            value["layer"]["parent"] = json!(base);
            layers.push(value);
            Ok(())
        }
        //403770: background, then the 68A668 overlay placement.
        match &visual.background {
            Some(name) => put(&mut layers, self.picture(&char_image(name), false, base + 1, [ox, oy], None, 1), base)?,
            None => layers.push(json!({"op":"put","layer":{"id":base+1,"parent":base,"rect":[0,0,800,600],"paint":{"kind":"solid","color":[0,0,0,255]},"order":[1,0]}})),
        }
        match &visual.overlay {
            Some((name, mode, offset)) => {
                let at = match mode {
                    1 => [ox, oy + 600],
                    2 => [ox, oy - 600],
                    3 => [ox - offset, oy],
                    _ => [ox + 800, oy],
                };
                put(&mut layers, self.picture(&char_image(name), false, base + 2, at, None, 2), base)?;
            }
            None => layers.push(hidden(base + 2, base)),
        }
        //403AF0: four mask-blended sprites (428350/427F10).
        for (slot, sprite) in visual.sprites.iter().enumerate() {
            let id = base + 10 + slot as u32;
            if !sprite.present {
                layers.push(hidden(id, base));
                continue;
            }
            let mask = self
                .images
                .get(&(char_image(&sprite.mask()), Kind::Opaque))
                .context("DC1 sprite mask not prepared")?;
            let mut value = self.picture(
                &char_image(sprite.image()),
                false,
                id,
                [sprite.x + shake[0], sprite.y + shake[1]],
                None,
                10 + slot as i32,
            )?;
            //Mask blue 0 keeps the sprite, 255 keeps the destination, other
            //values blend by 255 - red.
            value["layer"]["filter"]["mask"] = json!({"asset":mask.upload.path.relative(),"channel":"red","scale":-1.0,"offset":255.0});
            value["layer"]["parent"] = json!(base);
            layers.push(value);
        }
        //403AB0: the colour-keyed char/bsteff overlay at 0,0 after slot 0.
        let effect = visual.effect.as_ref().filter(|_| visual.sprites[0].present);
        match effect {
            Some(name) => {
                let mut value = self.picture(name, true, base + 20, [shake[0], shake[1]], None, 10)?;
                value["layer"]["order"] = json!([10, 1]);
                value["layer"]["parent"] = json!(base);
                layers.push(value);
            }
            None => layers.push(hidden(base + 20, base)),
        }
        band(&mut layers, 100);
        Ok(layers)
    }

    fn glyph_layer(&mut self, host: &mut dyn HostServices, state: &mut StoryHost, id: u32, glyphs: &[Glyph], visible: bool) -> Result<Value> {
        let palette = state.story.tables.palette.clone();
        let mut out = Vec::new();
        let metrics = host.text_measurement(self.client)?;
        for pass in 0..2 {
            for glyph in glyphs {
                let ascent = match state.ascent.get(&glyph.size) {
                    Some(value) => *value,
                    None => {
                        let value = metrics.measure_simple(&self.config.font, f32::from(glyph.size), " ")?.ascent;
                        state.ascent.insert(glyph.size, value);
                        value
                    }
                };
                let (dx, dy, color) = if pass == 0 {
                    (glyph.shadow.0[0], glyph.shadow.0[1], glyph.shadow.1)
                } else {
                    (0, 0, glyph.color)
                };
                let rgb = palette.get(color as usize).copied().unwrap_or([255, 255, 255]);
                for character in glyph.text.chars() {
                    let cell = if character.is_ascii() || ('\u{ff61}'..='\u{ff9f}').contains(&character) {
                        u16::from(glyph.size) / 2
                    } else {
                        u16::from(glyph.size)
                    };
                    out.push(json!({"character":character.to_string(),"baseline":[(glyph.x+dx) as f32,(glyph.y+dy) as f32+ascent],"size":glyph.size,"cell_width":cell,"color":[rgb[0],rgb[1],rgb[2],255]}));
                }
            }
        }
        Ok(json!({"op":"put","layer":{"id":id,"parent":UI,"rect":[0,0,800,600],"visible":visible,"paint":{"kind":"glyph_run","font":self.config.font,"size":30,"color":[255,255,255,255],"glyphs":out},"order":[20,0]}}))
    }

    fn project(&mut self, host: &mut dyn HostServices, state: &mut StoryHost) -> Result<()> {
        let mut layers = Vec::new();
        if !state.created {
            //Replace the title presentation with the scenario stage.
            //Title layers stay owned by the title; the stage covers them.
            for id in (0..5u32).map(|i| ROOT + 10 + i).chain([ROOT + 3]).chain((0..4u32).map(|i| ROOT + 20 + i)) {
                layers.push(json!({"op":"set_visible","id":id,"visible":false}));
            }
            layers.push(json!({"op":"put","layer":{"id":STAGE,"parent":ROOT,"rect":[0,0,800,600],"visible":true,"paint":{"kind":"group"},"order":[5,0]}}));
            layers.push(json!({"op":"put","layer":{"id":UI,"parent":ROOT,"rect":[0,0,800,600],"visible":true,"paint":{"kind":"group"},"order":[20,0]}}));
            self.submit(host, layers, vec![])?;
            layers = Vec::new();
            state.created = true;
        }
        let story = &state.story;
        let shake = story.screen_shake;
        let tables = story.tables.clone();
        layers.extend(self.visual_layers(&story.current_visual(), STAGE, shake, &tables)?);
        if let Some(before) = state.wipe.clone() {
            if !state.wipe_started {
                //4250D0: the old picture is an independent retained subtree.
                let mut old = vec![json!({"op":"put","layer":{"id":BEFORE,"parent":ROOT,"rect":[0,0,800,600],"paint":{"kind":"group"},"order":[6,0]}})];
                old.extend(self.visual_layers(&before, BEFORE, [0, 0], &tables)?);
                self.submit(host, old, vec![])?;
                let grade = state.story.tables.grades[state.story.grade as usize].clone();
                let rule = self
                    .images
                    .get(&(grade, Kind::Inverted))
                    .context("DC1 grade rule not prepared")?
                    .upload
                    .path
                    .relative()
                    .to_owned();
                layers.push(json!({"op":"begin_crossfade","id":BEFORE,"source":STAGE,"duration_ms":510,"rule":{"asset":rule,"vague":255},"external":true}));
                state.wipe_started = true;
            }
            let progress = state.story.wipe.unwrap_or(512).clamp(0, 510) as u64;
            layers.push(json!({"op":"set_crossfade_elapsed","id":BEFORE,"elapsed_ms":progress}));
        }
        //403610(1): Messwin3 frame0 (alpha 128), frame1 and the four buttons.
        let story = &state.story;
        let frontend = self.data.as_ref().unwrap().frontend.clone();
        let window = story.window;
        let mut frame = self.atlas_picture(&frontend.bar, 0, WINDOW, [16, 384], 1)?;
        frame["layer"]["parent"] = json!(UI);
        //426E00 with BA9E48: BA9E44 alpha (0..256).
        let alpha = (state.story.settings.window_alpha.clamp(0, 256) * 255 / 256) as u8;
        frame["layer"]["opacity"] = json!(alpha);
        frame["layer"]["visible"] = json!(window);
        layers.push(frame);
        let mut side = self.atlas_picture(&frontend.bar, 1, SIDE, [696, 440], 2)?;
        side["layer"]["parent"] = json!(UI);
        side["layer"]["visible"] = json!(window);
        layers.push(side);
        let buttons = story.tables.window_buttons.clone();
        for (i, row) in buttons.iter().enumerate() {
            let shown = window && (i != 3 || story.read_scene());
            //408F70: row[6] while hovered, row[5] on the click frame.
            let frame = if story.window_pressed == Some(i) {
                row[5]
            } else if story.window_hover == Some(i) {
                row[6]
            } else {
                row[4]
            };
            let mut button = self.atlas_picture(&frontend.bar, frame as u32, BUTTONS + i as u32, [696 + row[0], 440 + row[1]], 3)?;
            button["layer"]["parent"] = json!(UI);
            button["layer"]["visible"] = json!(shown);
            layers.push(button);
        }
        let name_glyphs = state.story.name_glyphs.clone();
        let shown_name = state.story.window && state.story.name_shown;
        layers.push(self.glyph_layer(host, state, NAME, &name_glyphs, shown_name)?);
        let text = state.story.glyphs.clone();
        let window = state.story.window;
        layers.push(self.glyph_layer(host, state, TEXT, &text, window)?);
        let story = &state.story;
        match story.cursor {
            Some(frame) if story.window => {
                let mut cursor = self.atlas_picture(&story.tables.key_wait, frame, CURSOR, [672, 544], 30)?;
                cursor["layer"]["parent"] = json!(UI);
                layers.push(cursor);
            }
            _ => layers.push(hidden(CURSOR, UI)),
        }
        //406C30: Messwin3 frame2 rows at 200+56n, frame3 for the selection.
        let choice = state.story.choice.clone();
        for i in 0..15u32 {
            for part in 0..3 {
                layers.push(hidden(CHOICES + 3 * i + part, UI));
            }
        }
        if let Some(choice) = &choice {
            let longest = choice
                .items
                .iter()
                .enumerate()
                .filter(|(i, _)| choice.enabled[*i])
                .map(|(_, item)| item.1.len())
                .max()
                .unwrap_or(0);
            let size = if longest > 28 { 24 } else { 30 };
            for (i, item) in choice.items.iter().enumerate() {
                if !choice.enabled[i] {
                    continue;
                }
                let y = 200 + 56 * i as i32;
                let base = CHOICES + 3 * i as u32;
                let mut row = self.atlas_picture(&frontend.bar, 2, base, [200, y], 40)?;
                row["layer"]["parent"] = json!(UI);
                row["layer"]["opacity"] = json!((state.story.settings.window_alpha.clamp(0, 256) * 255 / 256) as u8);
                layers.push(row);
                if choice.selected == i {
                    let mut lit = self.atlas_picture(&frontend.bar, 3, base + 1, [200, y], 41)?;
                    lit["layer"]["parent"] = json!(UI);
                    layers.push(lit);
                }
                //412DD0: GDI TextOut, 0x101010 shadow at +1,+1, then white.
                let text = crate::dc1_story::decode(&item.1);
                let mut glyphs = Vec::new();
                let mut x = 232;
                for character in text.chars() {
                    let cell = if character.is_ascii() { size / 2 } else { size };
                    glyphs.push(Glyph { x, y: y + 8, size: size as u8, text: character.to_string(), color: 0, shadow: ([1, 1], 2) });
                    x += cell;
                }
                let mut run = self.glyph_layer(host, state, base + 2, &glyphs, true)?;
                run["layer"]["order"] = json!([42, 0]);
                layers.push(run);
            }
        }
        let story = &state.story;
        //40BB30 runs only from 408F70 modes 0/11/15/17/18, never inside a wipe.
        match story.place {
            Some((frame, alpha)) if alpha > 0 && story.wipe.is_none() => {
                let atlas = story.tables.map_name.clone();
                let mut place = self.atlas_picture(&atlas, frame as u32, PLACE, [576, 24], 50)?;
                place["layer"]["parent"] = json!(UI);
                place["layer"]["opacity"] = json!((alpha.clamp(0, 256) * 255 / 256) as u8);
                layers.push(place);
            }
            _ => layers.push(hidden(PLACE, UI)),
        }
        for layer in &mut layers {
            if layer["layer"]["parent"] == json!(UI) {
                band(std::slice::from_mut(layer), 200);
            }
        }
        layers.extend(self.eyecatch_layers(state)?);
        self.submit(host, layers, vec![])?;
        self.system_layers(host, state)
    }

    /// 40B7B0/40BE70 backdrop: 429470(0) halves every channel; the date card
    /// (403F50) and panel follow.
    fn system_layers(&mut self, host: &mut dyn HostServices, state: &mut StoryHost) -> Result<()> {
        let story = &state.story;
        let tables = story.tables.clone();
        let bar = self.data.as_ref().unwrap().frontend.bar.clone();
        //27/31/32 draw over the menu still on the back buffer.
        let menu = match story.mode {
            26 => story.menu.clone(),
            27 | 31 | 32 => Some(crate::dc1_story::SystemMenu::default()),
            _ => None,
        };
        let dialog = story.dialog.clone();
        let backlog = story.backlog;
        let option = story.option.clone().filter(|_| story.mode == 27);
        let active = menu.is_some() || dialog.is_some() || backlog.is_some() || option.is_some();
        let mut layers = vec![json!({"op":"put","layer":{"id":SYSTEM,"parent":ROOT,"rect":[0,0,800,600],"visible":active,"paint":{"kind":"group"},"order":[50,0]}})];
        if !active {
            if state.option_shown {
                state.option_shown = false;
                self.option_layers_hide(host)?;
            }
            return self.submit(host, layers, vec![]);
        }
        layers.push(json!({"op":"put","layer":{"id":SYSTEM+1,"parent":SYSTEM,"rect":[0,0,800,600],"visible":option.is_none(),"paint":{"kind":"solid","color":[0,0,0,128]},"order":[1,0]}}));
        let mut parts = Vec::new();
        if menu.is_some() || dialog.is_some() {
            //40B7B0: date card at 24,24 and AdvOptChip frame 0 at 496,0.
            let [month, day, weekday] = [story.date[1], story.date[2], story.time_of_day[0]];
            let mut card = vec![([0, 0], 17)];
            if month < 10 {
                card.push(([32, 8], month + 25));
            } else {
                card.push(([16, 8], 26));
                card.push(([32, 8], month + 15));
            }
            if day < 10 {
                card.push(([88, 8], day + 25));
            } else {
                card.push(([72, 8], day / 10 + 25));
                card.push(([88, 8], day % 10 + 25));
            }
            card.push(([136, 8], weekday + 18));
            for (i, (at, frame)) in card.iter().enumerate() {
                parts.push(self.atlas_picture(&bar, *frame as u32, SYSTEM + 2 + i as u32, [24 + at[0], 24 + at[1]], 2)?);
            }
            parts.push(self.atlas_picture(&tables.menu_chip, 0, SYSTEM + 10, [496, 0], 3)?);
            let hover = menu.as_ref().and_then(|menu| menu.hover);
            let chosen = menu.as_ref().and_then(|menu| menu.chosen);
            for (i, row) in tables.menu_rows.iter().enumerate() {
                //40B7E0: row[4] normal, row[6] hovered, row[5] once chosen.
                let frame = if chosen == Some(i) {
                    row[5]
                } else if hover == Some(i) {
                    row[6]
                } else {
                    row[4]
                };
                parts.push(self.atlas_picture(&tables.menu_chip, frame as u32, SYSTEM + 11 + i as u32, [row[0] + 496, row[1]], 4)?);
            }
        }
        if let Some(dialog) = &dialog {
            //414500: YesNoWin frame 6, prompt at +8,+8 (h[0], 24 px), rows.
            let [x, y] = dialog.at;
            parts.push(self.atlas_picture(&tables.dialog_chip, 6, SYSTEM + 20, [x, y], 6)?);
            for (i, row) in tables.dialog_rows.iter().enumerate() {
                let frame = if dialog.hover == i as i32 { row[5] } else { row[4] };
                parts.push(self.atlas_picture(&tables.dialog_chip, frame as u32, SYSTEM + 21 + i as u32, [x + row[0], y + row[1]], 7)?);
            }
            let text = crate::dc1_story::decode(&dialog.prompt);
            let mut glyphs = Vec::new();
            let mut cx = x + 8;
            for character in text.chars() {
                glyphs.push(Glyph { x: cx, y: y + 8, size: 24, text: character.to_string(), color: 0, shadow: ([1, 1], 2) });
                cx += if character.is_ascii() { 12 } else { 24 };
            }
            let mut run = self.glyph_layer(host, state, SYSTEM + 23, &glyphs, true)?;
            run["layer"]["order"] = json!([8, 0]);
            parts.push(run);
        }
        if backlog.is_some() {
            let glyphs = state.story.backlog_glyphs.clone();
            let mut run = self.glyph_layer(host, state, SYSTEM + 30, &glyphs, true)?;
            run["layer"]["order"] = json!([5, 0]);
            parts.push(run);
            //40BE70: Messwin3 frame 1 at 696,440 and the 440A60 buttons.
            parts.push(self.atlas_picture(&bar, 1, SYSTEM + 31, [696, 440], 6)?);
            let hover = state.story.window_hover;
            for (i, row) in tables.backlog_rows.iter().enumerate() {
                let frame = if hover == Some(i) { row[6] } else { row[4] };
                parts.push(self.atlas_picture(&bar, frame as u32, SYSTEM + 32 + i as u32, [696 + row[0], 440 + row[1]], 7)?);
            }
        }
        for part in &mut parts {
            part["layer"]["parent"] = json!(SYSTEM);
        }
        band(&mut parts, 500);
        layers[1]["layer"]["order"] = json!([500, 0]);
        let drawn: BTreeSet<u32> = parts
            .iter()
            .filter_map(|part| part["layer"]["id"].as_u64().map(|id| id as u32))
            .collect();
        for stale in state.system_ids.difference(&drawn) {
            layers.push(json!({"op":"set_visible","id":stale,"visible":false}));
        }
        state.system_ids = drawn;
        layers.extend(parts);
        self.submit(host, layers, vec![])?;
        if let Some(screen) = option {
            let tables = self.data.as_ref().unwrap().frontend.config.clone();
            for (name, key) in [(&tables.back, true), (&tables.chip.path, true), (&tables.sys_chip.path, true)] {
                if !self.image(host, name, key)? {
                    return Ok(());
                }
            }
            let settings = state.story.settings.clone();
            self.option_layers_for(host, &tables, &screen, &settings, true)?;
            state.option_shown = true;
        }
        Ok(())
    }

    /// 4030D0 with 68A640 != -1: IcatchBack at 0,0 and the 6855B0/B4/B8
    /// month, day and weekday from IcatchChip at a 43D1C0 layout.
    fn eyecatch_layers(&self, state: &StoryHost) -> Result<Vec<Value>> {
        let story = &state.story;
        let Some(heroine) = story.eyecatch else {
            return Ok(vec![
                json!({"op":"put","layer":{"id":EYECATCH,"parent":ROOT,"rect":[0,0,800,600],"visible":false,"paint":{"kind":"group"},"order":[40,0]}}),
            ]);
        };
        let (back, chip) = &story.tables.eyecatches[heroine];
        let layout = story.tables.eyecatch_layouts[[2, 2, 0, 0, 1, 1, 3, 3][heroine]];
        let mut layers = vec![
            json!({"op":"put","layer":{"id":EYECATCH,"parent":ROOT,"rect":[0,0,800,600],"visible":true,"paint":{"kind":"group"},"order":[40,0]}}),
        ];
        let mut part = |id: u32, value: Result<Value>| -> Result<()> {
            let mut value = value?;
            value["layer"]["parent"] = json!(EYECATCH);
            layers.push(value);
            let _ = id;
            Ok(())
        };
        part(1, self.picture(back, false, EYECATCH + 1, [0, 0], None, 1))?;
        let [month, day, weekday] = [story.date[1], story.date[2], story.time_of_day[0]];
        ensure!(
            (0..10).contains(&month) && (0..100).contains(&day) && (0..7).contains(&weekday),
            "DC1 eyecatch date fields"
        );
        let at = |n: usize| [layout[2 * n], layout[2 * n + 1]];
        let mut cells = vec![(at(0), month), (at(1), 10)];
        if day < 10 {
            cells.push((at(3), day));
        } else {
            cells.push((at(2), day / 10));
            cells.push((at(3), day % 10));
        }
        cells.push((at(4), 11));
        cells.push((at(5), weekday + 13));
        let count = cells.len();
        for (i, (position, frame)) in cells.into_iter().enumerate() {
            part(0, self.atlas_picture(chip, frame as u32, EYECATCH + 2 + i as u32, position, 2))?;
        }
        if count == 5 {
            layers.push(hidden(EYECATCH + 7, EYECATCH));
        }
        band(&mut layers[1..], 400);
        Ok(layers)
    }

    /// Image preparation with the three native load profiles.
    pub(super) fn image_kind(&mut self, host: &mut dyn HostServices, name: &str, kind: Kind) -> Result<bool> {
        self.image_with(host, name, kind)
    }
}
