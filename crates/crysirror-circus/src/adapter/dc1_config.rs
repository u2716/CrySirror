//! DC.EXE title selection (414070/413CB0) and option screen (407CC0/4083E0)
//! projected over shared Layer/Image/Audio services.
use super::*;
use crate::dc1_config::{ConfigAction, ConfigInput, ConfigScreen};
use crysirror_core::service::{AudioParameters, AudioPreparePhase, PcmBus, buttons as b};

const TITLE_SUB: u32 = ROOT + 20;
const OPTION: u32 = ROOT + 0x5000;
/// 413CB0 submenu entry positions for rows 5..7 and 8..9.
const EXTRA_AT: [[i32; 2]; 3] = [[192, 368], [176, 400], [160, 432]];
const CONFIG_AT: [[i32; 2]; 2] = [[144, 472], [128, 512]];

/// Native input flags from 418F70 for one host frame.
pub(super) struct Flags {
    pub(super) x: i32,
    pub(super) y: i32,
    /// BA9F50/BA9F58: left button.
    pub(super) click: bool,
    /// BA9F48: Space.
    pub(super) space: bool,
    /// BA9F4C: Enter.
    pub(super) enter: bool,
    /// BA9F70: right button, Escape or Backspace.
    pub(super) cancel: bool,
}
impl Flags {
    pub(super) fn new(input: InputFrame) -> Self {
        let pressed = input.buttons_pressed;
        Self {
            x: input.pointer_x,
            y: input.pointer_y,
            click: pressed & b::MOUSE_LEFT != 0,
            space: pressed & b::SPACE != 0,
            enter: pressed & b::ENTER != 0,
            cancel: pressed & (b::MOUSE_RIGHT | b::ESCAPE | b::BACKSPACE) != 0,
        }
    }
}

impl Dc1Vm {
    /// 42A5B0(id, 0) on the SE channel with the configured BA9E28 level.
    pub(super) fn system_sound(&mut self, host: &mut dyn HostServices, id: i32) -> Result<()> {
        if !self.settings.sound.0 {
            return Ok(());
        }
        let name = self
            .data
            .as_ref()
            .unwrap()
            .system
            .sound
            .get(&id)
            .context("DC1 System.MES sound index")?
            .replace('\\', "/");
        if let Some(ticket) = self.system_audio.take() {
            host.release_audio(self.client, ticket)?;
        }
        let gain = self.settings.sound.1 - 1000;
        self.system_audio = Some(host.begin_audio(
            self.client,
            AudioRequest::PlayAsset {
                channel: 1,
                source: crysirror_core::service::AudioAsset {
                    path: VfsPath::new(Mount::Asset, &name)?,
                    frame_limit: None,
                },
                intro: None,
                parameters: AudioParameters {
                    gain: 10f32.powf(gain.max(-10000) as f32 / 2000.),
                    bus: PcmBus::Sfx,
                    paused: true,
                    ..Default::default()
                },
            },
        )?);
        Ok(())
    }
    pub(super) fn poll_system_audio(&mut self, host: &mut dyn HostServices) -> Result<()> {
        let Some(ticket) = self.system_audio else {
            return Ok(());
        };
        let prepared = host.audio_preparation(self.client, ticket)?;
        match prepared.phase {
            AudioPreparePhase::Pending => {}
            AudioPreparePhase::Failed => bail!(
                "DC1 system sound: {}",
                prepared.error.unwrap_or_else(|| "unknown error".into())
            ),
            AudioPreparePhase::Ready => {
                host.submit_audio(self.client, AudioRequest::Pause { channel: 1, paused: false, fade_ms: 0. })?;
                host.release_audio(self.client, ticket)?;
                self.system_audio = None;
            }
        }
        Ok(())
    }

    /// 414070 hit testing plus 413CB0 drawing. Returns the confirmed action.
    pub(super) fn title_select(&mut self, host: &mut dyn HostServices, input: InputFrame) -> Result<Option<u32>> {
        self.poll_system_audio(host)?;
        let front = self.data.as_ref().unwrap().frontend.clone();
        for (name, key) in [
            (&front.background, false),
            (&front.title.path, true),
            (&front.extra.path, true),
        ] {
            if !self.image(host, name, key)? {
                return Ok(None);
            }
        }
        let flags = Flags::new(input);
        let (x, y) = (flags.x, flags.y);
        let inside = |row: &crate::dc1_frontend::Button| {
            let [left, top, width, height] = row.bounds;
            x >= left && y >= top && x < left + width && y < top + height
        };
        let mut pointer_select = false;
        if let Some(index) = front.buttons[..5].iter().position(inside) {
            if self.hover != Some(index as u32) {
                self.system_sound(host, 28)?;
                self.hover = Some(index as u32);
            }
            pointer_select |= flags.click;
        }
        let submenu = match self.hover {
            Some(2) => 1,
            Some(3) => 2,
            _ => 0,
        };
        if submenu != 0 {
            let rows = if submenu == 1 { 5..8 } else { 8..10 };
            let first = rows.start;
            if let Some(offset) = front.buttons[rows].iter().position(inside) {
                if self.sub_hover != Some(offset as u32) {
                    self.system_sound(host, 28)?;
                    self.sub_hover = Some(offset as u32);
                }
                pointer_select |= flags.click;
            }
            let _ = first;
        } else {
            self.sub_hover = None;
        }
        let mut confirmed = None;
        if (pointer_select || flags.space || flags.enter) && self.hover.is_some() {
            let hover = self.hover.unwrap();
            match (hover, self.sub_hover) {
                (2, Some(sub)) => confirmed = Some(5 + sub),
                (3, Some(sub)) => confirmed = Some(8 + sub),
                (2 | 3, None) => {}
                (other, _) => confirmed = Some(other),
            }
            if confirmed.is_some() {
                self.system_sound(host, 19)?;
                self.pressed = confirmed;
            }
        }
        let mut layers = vec![self.picture(&front.background, false, ROOT + 3, [0, 0], None, 1)?];
        for (i, button) in front.buttons[..5].iter().enumerate() {
            let frame = if self.pressed == Some(i as u32) {
                button.frames[1]
            } else if self.hover == Some(i as u32) {
                button.frames[2]
            } else {
                button.frames[0]
            };
            layers.push(self.atlas_picture(&front.title, frame, ROOT + 10 + i as u32, [button.bounds[0], button.bounds[1]], 2)?);
        }
        //413CB0: the TitleChip02 panel, then hovered (row[5]) or confirmed
        //(row[6]) entries at fixed positions.
        let entries: Vec<(usize, [i32; 2])> = match submenu {
            1 => (0..3).map(|k| (5 + k, EXTRA_AT[k])).collect(),
            2 => (0..2).map(|k| (8 + k, CONFIG_AT[k])).collect(),
            _ => Vec::new(),
        };
        match submenu {
            0 => layers.push(json!({"op":"put","layer":{"id":TITLE_SUB,"parent":ROOT,"rect":[0,0,1,1],"visible":false,"paint":{"kind":"solid","color":[0,0,0,0]}}})),
            n => layers.push(self.atlas_picture(&front.extra, n - 1, TITLE_SUB, if n == 1 { [0, 368] } else { [0, 464] }, 3)?),
        }
        for k in 0..3usize {
            let id = TITLE_SUB + 1 + k as u32;
            let entry = entries.get(k).and_then(|(row, at)| {
                let frames = front.buttons[*row].frames;
                if self.pressed == Some(*row as u32) {
                    Some((frames[2], *at))
                } else if self.sub_hover == Some(k as u32) {
                    Some((frames[1], *at))
                } else {
                    None
                }
            });
            match entry {
                Some((frame, at)) => layers.push(self.atlas_picture(&front.title, frame, id, at, 4)?),
                None => layers.push(json!({"op":"put","layer":{"id":id,"parent":ROOT,"rect":[0,0,1,1],"visible":false,"paint":{"kind":"solid","color":[0,0,0,0]}}})),
            }
        }
        self.submit(host, layers, vec![])?;
        Ok(confirmed)
    }

    pub(super) fn enter_config(&mut self, host: &mut dyn HostServices) -> Result<()> {
        //407CC0(-1): 6B0DB8 = 6B0DC0 = 0 and a full 4083E0(1) draw.
        self.option = Some(ConfigScreen::default());
        self.option_visible(host, true)
    }
    pub(super) fn option_layers_hide(&mut self, host: &mut dyn HostServices) -> Result<()> {
        self.option_visible(host, false)
    }
    fn option_visible(&mut self, host: &mut dyn HostServices, visible: bool) -> Result<()> {
        self.submit(host, vec![json!({"op":"put","layer":{"id":OPTION,"parent":ROOT,"rect":[0,0,800,600],"visible":visible,"paint":{"kind":"group"},"order":[60,0]}})], vec![])
    }

    /// 413250 state 11: 407CC0(0) until it returns 1.
    pub(super) fn config_frame(&mut self, host: &mut dyn HostServices, input: InputFrame) -> Result<bool> {
        self.poll_system_audio(host)?;
        let tables = self.data.as_ref().unwrap().frontend.config.clone();
        for (name, key) in [(&tables.back, true), (&tables.chip.path, true), (&tables.sys_chip.path, true)] {
            if !self.image(host, name, key)? {
                return Ok(false);
            }
        }
        let flags = Flags::new(input);
        let mut screen = self.option.take().context("DC1 option state")?;
        let action = screen.input(
            &tables,
            &mut self.settings,
            &ConfigInput {
                x: flags.x,
                y: flags.y,
                pointer: true,
                select: flags.click || flags.space || flags.enter,
                cancel: flags.cancel,
            },
        );
        self.option = Some(screen);
        match action {
            ConfigAction::PreviewSound => self.system_sound(host, 19)?,
            ConfigAction::MusicOff => {
                host.submit_audio(self.client, AudioRequest::Stop { channel: 0, fade_ms: 0. })?;
            }
            ConfigAction::VoiceOff => {
                host.submit_audio(self.client, AudioRequest::Stop { channel: 2, fade_ms: 0. })?;
            }
            ConfigAction::NameEntry => bail!("DC1 41E180 player-name entry is not ported"),
            ConfigAction::Close => {
                self.option = None;
                self.option_visible(host, false)?;
                return Ok(true);
            }
            //Previews of a voice sample and the title BGM level change take
            //effect through the channel gains on the next playback.
            ConfigAction::PreviewVoice | ConfigAction::MusicLevel | ConfigAction::Redraw | ConfigAction::None => {}
        }
        let screen = self.option.clone().context("DC1 option state")?;
        let settings = self.settings.clone();
        self.option_layers_for(host, &tables, &screen, &settings, true)?;
        Ok(false)
    }

    /// 4083E0 over the shared option group (title and in-game 408F70 27).
    pub(super) fn option_layers_for(
        &mut self,
        host: &mut dyn HostServices,
        tables: &crate::dc1_frontend::ConfigTables,
        screen: &ConfigScreen,
        settings: &Settings,
        visible: bool,
    ) -> Result<()> {
        self.option_visible(host, visible)?;
        let screen = screen.clone();
        let settings = settings.clone();
        let mut layers = vec![self.picture(&tables.back, true, OPTION + 1, [0, 0], None, 1)?];
        let row = |i: usize| tables.rows[i];
        let hover = screen.hover;
        let toggles = [
            settings.voice_special,
            settings.voice_other,
            settings.effect_mode,
            settings.option3,
            settings.seen_skip,
        ];
        for (i, on) in toggles.iter().enumerate() {
            let r = row(i);
            let frame = if i == 2 && !settings.effect_capable {
                r[4] + 6
            } else {
                (if *on { r[5] } else { r[4] }) + if hover == i as i32 { 3 } else { 0 }
            };
            layers.push(self.atlas_picture(&tables.chip, frame as u32, OPTION + 2 + i as u32, [r[0], r[1]], 2)?);
        }
        let bars = [
            (5, Settings::level_width(settings.music.0, settings.music.1)),
            (6, Settings::level_width(settings.voice.0, settings.voice.1)),
            (7, Settings::level_width(settings.sound.0, settings.sound.1)),
            (9, settings.speed_width()),
            (10, settings.alpha_width()),
        ];
        for (n, (i, width)) in bars.iter().enumerate() {
            let r = row(*i);
            let id = OPTION + 10 + 3 * n as u32;
            layers.push(self.atlas_picture(&tables.chip, r[4] as u32, id, [r[0], r[1]], 2)?);
            //41BEF0 clips frame 45 to the value width; frame 46 is the knob.
            let bar = *tables.chip.frames.get(45).context("DC1 OptChip0 bar frame")?;
            let visible = (*width).min(bar[2]);
            if visible > 0 {
                layers.push(self.picture(&tables.chip.path, true, id + 1, [r[0] + 168, r[1] + 16], Some([bar[0], bar[1], visible, bar[3]]), 3)?);
            } else {
                layers.push(json!({"op":"put","layer":{"id":id+1,"parent":ROOT,"rect":[0,0,1,1],"visible":false,"paint":{"kind":"solid","color":[0,0,0,0]}}}));
            }
            layers.push(self.atlas_picture(&tables.chip, 46, id + 2, [r[0] + 168 + (*width).min(156), r[1] + 12], 4)?);
        }
        let r = row(8);
        layers.push(self.atlas_picture(&tables.chip, r[4] as u32, OPTION + 30, [r[0], r[1]], 2)?);
        for (cell, radio) in tables.radio.iter().enumerate() {
            let mut frame = if cell as i32 == settings.text_mode { radio[5] } else { radio[4] };
            if hover == 8 && cell as i32 == screen.radio_hover {
                frame = radio[6];
            }
            layers.push(self.atlas_picture(&tables.chip, frame as u32, OPTION + 31 + cell as u32, [r[0] + radio[0], r[1] + radio[1]], 3)?);
        }
        let r = row(11);
        let frame = if hover == 11 { r[5] } else { r[4] };
        layers.push(self.atlas_picture(&tables.chip, frame as u32, OPTION + 40, [r[0], r[1]], 2)?);
        let r = row(12);
        layers.push(self.atlas_picture(&tables.sys_chip, if hover == 12 { 101 } else { 100 }, OPTION + 41, [r[0], r[1]], 2)?);
        for layer in &mut layers {
            layer["layer"]["parent"] = json!(OPTION);
        }
        super::story::band(&mut layers, 600);
        self.submit(host, layers, vec![])
    }
}
use crate::dc1_config::Settings;
