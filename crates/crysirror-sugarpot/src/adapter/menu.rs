// SPDX-License-Identifier: MPL-2.0
//! Native settings values presented by shared retained, modal UI controls.
use super::*;
const PANEL: u32 = 64000;
const CLOSE: u32 = 64001;
#[derive(Clone, Copy)]
enum Setting {
    Header(usize, i32),
    AutoDelay,
}
const SETTINGS: [(&str, Setting); 11] = [
    ("画面モード", Setting::Header(0, 2)),
    ("演出の簡略化", Setting::Header(4, 1)),
    ("メッセージ速度", Setting::Header(8, 3)),
    ("自動送り待ち時間", Setting::AutoDelay),
    ("既読のみスキップ", Setting::Header(10, 1)),
    ("ＢＧＭ", Setting::Header(1, 1)),
    ("ＢＧＭ音量", Setting::Header(5, 100)),
    ("音声", Setting::Header(2, 1)),
    ("音声音量", Setting::Header(6, 100)),
    ("効果音", Setting::Header(3, 1)),
    ("効果音音量", Setting::Header(7, 100)),
];
impl SugarPotVm {
    pub(super) fn settings_menu(&mut self, h: &mut dyn HostServices) -> Result<()> {
        self.settings_closing = false;
        let mut ui = vec![
            json!({"op":"put","widget":{"id":PANEL,"rect":[112,30,800,540],"kind":"panel","z":10000,"modal":true,"color":[20,24,32,250]}}),
            json!({"op":"put","widget":{"id":CLOSE,"parent":PANEL,"rect":[635,480,140,42],"kind":"button","text":"閉じる","font":self.config.font,"text_size":22}}),
        ];
        for (index, (label, setting)) in SETTINGS.iter().enumerate() {
            let y = 20 + index as i32 * 40;
            let value = match setting {
                Setting::Header(field, max) => self.settings.header[*field] as f32 / *max as f32,
                Setting::AutoDelay => self.settings.values[6] as f32 / 200.,
            };
            let value = if matches!(setting, Setting::Header(5 | 6 | 7, 100)) {
                1. - value
            } else {
                value
            };
            ui.push(json!({"op":"put","widget":{"id":64100+index as u32,"parent":PANEL,"rect":[20,y,270,36],"kind":"label","text":label,"font":self.config.font,"text_size":22}}));
            ui.push(json!({"op":"put","widget":{"id":64200+index as u32,"parent":PANEL,"rect":[310,y,450,36],"kind":if matches!(setting,Setting::Header(_,1)){"toggle"}else{"slider"},"value":value,"font":self.config.font}}));
        }
        self.submit(h, vec![], ui)?;
        self.wait = Some(Wait::Settings);
        Ok(())
    }
    pub(super) fn poll_settings(
        &mut self,
        h: &mut dyn HostServices,
        input: &mut InputFrame,
    ) -> Result<bool> {
        if let Some((source, channel, bus, looping)) = self.menu_audio.first().cloned() {
            if !self.play_audio(h, &source, channel, bus, looping)? {
                return Ok(false);
            }
            self.menu_audio.remove(0);
        }
        let events: Vec<Json> =
            serde_json::from_slice(&h.query_engine(self.client, EngineQuery::UiEvents)?)?;
        let mut close = self.settings_closing || input.buttons_pressed & buttons::ESCAPE != 0;
        for event in events {
            if event["widget"]["owner"].as_u64() != Some(self.client.0 as u64) {
                continue;
            }
            let Some(id) = event["widget"]["id"].as_u64() else {
                continue;
            };
            if id == CLOSE as u64 && event["kind"] == "activate" {
                close = true;
                continue;
            }
            if event["kind"] != "change" || !(64200..64200 + SETTINGS.len() as u64).contains(&id) {
                continue;
            }
            let index = (id - 64200) as usize;
            let mut value = event["value"]
                .as_f64()
                .context("SugarPot setting UI value")?
                .clamp(0., 1.);
            match SETTINGS[index].1 {
                Setting::Header(field, max) => {
                    if matches!(field, 5 | 6 | 7) {
                        value = 1. - value;
                    }
                    self.settings.header[field] = (value * max as f64).round() as i32;
                    if field == 0 {
                        self.submit(
                            h,
                            vec![],
                            vec![
                                json!({"op":"set_fullscreen","enabled":self.settings.header[0]!=0}),
                            ],
                        )?;
                    }
                    if matches!(field, 1 | 2 | 3) {
                        if self.settings.header[field] == 0 {
                            let channels = match field {
                                1 => vec![0],
                                2 => vec![2],
                                _ => (vec![1]).into_iter().chain(3..19).collect(),
                            };
                            for channel in channels {
                                self.stop_audio(h, channel, 0)?;
                            }
                        } else if field == 1 && self.last_music_loop && !self.last_music.is_empty()
                        {
                            self.menu_audio
                                .push((self.last_music.clone(), 0, PcmBus::Music, true));
                        } else if field == 3 && self.last_env_loop && !self.last_env.is_empty() {
                            self.menu_audio
                                .push((self.last_env.clone(), 1, PcmBus::Ambient, true));
                        }
                    }
                    if matches!(field, 5 | 6 | 7) {
                        let bus = match field {
                            5 => PcmBus::Music,
                            6 => PcmBus::Voice,
                            _ => PcmBus::Sfx,
                        };
                        let gain = self.native_gain(bus)?;
                        let channels = match field {
                            5 => vec![0],
                            6 => vec![2],
                            _ => (vec![1]).into_iter().chain(3..19).collect(),
                        };
                        for channel in channels {
                            if h.audio_info(self.client, channel)?.is_some() {
                                h.submit_audio(
                                    self.client,
                                    AudioRequest::SetGain { channel, gain, fade_ms: 0. },
                                )?;
                            }
                        }
                    }
                }
                Setting::AutoDelay => {
                    let pct = (100. * value).round() as i32;
                    self.settings.values[6] = 200 * pct / 100;
                    self.settings.values[7] = 4 * pct / 100;
                }
            }
        }
        if !close {
            return Ok(false);
        }
        self.settings_closing = true;
        if !self.menu_audio.is_empty() {
            return Ok(false);
        }
        if !self.persist_preferences("settings")? {
            return Ok(false);
        }
        self.submit(h, vec![], vec![json!({"op":"remove","id":PANEL})])?;
        self.settings_closing = false;
        input.buttons_pressed = 0;
        Ok(true)
    }
    pub(super) fn native_gain(&self, bus: PcmBus) -> Result<f32> {
        let (enabled, volume) = match bus {
            PcmBus::Music => (self.settings.header[1], self.settings.header[5]),
            PcmBus::Voice => (self.settings.header[2], self.settings.header[6]),
            _ => (self.settings.header[3], self.settings.header[7]),
        };
        if enabled != 1 {
            return Ok(0.);
        }
        ensure!(
            (0..=100).contains(&volume),
            "SugarPot native attenuation setting"
        );
        let db = if volume == 100 {
            -10000
        } else {
            (-(volume * volume) as f64 * 0.300000011920929) as i32
        };
        Ok(crysirror_audio::levels::gain_from_legacy_db100(db)?)
    }
}
