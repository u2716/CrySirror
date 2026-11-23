// SPDX-License-Identifier: MPL-2.0
//! `ConfigMgr`/`ConfigCtrl`: the `__GameMain__` config panel (open/close
//! fade, OK) with the `CMSystem` message, model-transparency and sound
//! sliders. Screen settings (resolution, full screen, anti-aliasing,
//! shadow/texture quality, bloom, capture size) have no host counterpart in
//! this port; their buttons are recorded only (SATD-cm3d2-game-logic).
use crate::{adapter::Game, sound::Kind};
use anyhow::{Context, Result};

const PANEL: &str = "__GameMain__/SystemUI Root/ConfigPanel";

/// `CMSystem` values the panel edits (percent).
#[derive(Clone, Debug, PartialEq)]
pub struct Config {
    pub open: bool,
    /// `CMSystem.MsgTextSpeed`.
    pub text_speed: i32,
    /// `CMSystem.MsgAutoSpeed`.
    pub auto_speed: i32,
    /// `CMSystem.SysButtonShowAlways`.
    pub sys_button_show_always: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self { open: false, text_speed: 50, auto_speed: 50, sys_button_show_always: false }
    }
}

/// `ConfigMgr.SliderType` handlers: (slider path, handler).
const SLIDERS: [(&str, &str); 10] = [
    ("Screen/PlayerModelTransparency/Slider", "ConfigMgr.OnPlayerModelTransparencyChange"),
    ("Screen/Bloom/Slider", "ConfigMgr.OnBloomChange"),
    ("Message/MessageWindowTransparency/Slider", "ConfigMgr.OnMessageWindowTransparencyChange"),
    ("Message/TextSpeed/Slider", "ConfigMgr.OnTextSpeedChange"),
    ("Message/AutoSpeed/Slider", "ConfigMgr.OnAutoSpeedChange"),
    ("Sound/MasterVolume/Slider", "ConfigMgr.OnMasterVolumeChange"),
    ("Sound/SystemVolume/Slider", "ConfigMgr.OnSystemVolumeChange"),
    ("Sound/MusicVolume/Slider", "ConfigMgr.OnMusicVolumeChange"),
    ("Sound/VoiceVolume/Slider", "ConfigMgr.OnVoiceVolumeChange"),
    ("Sound/SoundEffectVolume/Slider", "ConfigMgr.OnSoundEffectVolumeChange"),
];
const ENV_SLIDER: (&str, &str) = ("Sound/SoundEnvironmentVolume/Slider", "ConfigMgr.OnSoundEnvironmentVolumeChange");
/// On/off and toggle buttons registered by `ConfigCtrl.Init`.
const BUTTONS: [(&str, &str); 12] = [
    ("System/SysButtonShowAlways/On", "ConfigMgr.OnSysButtonShowAlwaysEnabled"),
    ("System/SysButtonShowAlways/Off", "ConfigMgr.OnSysButtonShowAlwaysDisabled"),
    ("Screen/FullScreen/On", "ConfigMgr.OnFullScreenEnabled"),
    ("Screen/FullScreen/Off", "ConfigMgr.OnFullScreenDisabled"),
    ("Screen/FpsDisplay/On", "ConfigMgr.OnFpsDisplayEnabled"),
    ("Screen/FpsDisplay/Off", "ConfigMgr.OnFpsDisplayDisabled"),
    ("Message/SequentialVoice/On", "ConfigMgr.OnSequentialVoiceEnabled"),
    ("Message/SequentialVoice/Off", "ConfigMgr.OnSequentialVoiceDisabled"),
    ("Sound/MusicVolume/3D", "ConfigMgr.OnMusic3DActive"),
    ("Sound/VoiceVolume/3D", "ConfigMgr.OnVoice3DActive"),
    ("Sound/SoundEffectVolume/3D", "ConfigMgr.OnSoundEffect3DActive"),
    ("Sound/SoundEnvironmentVolume/3D", "ConfigMgr.OnSoundEnvironment3DActive"),
];

impl Game {
    fn config_node(&self, path: &str) -> Result<usize> {
        self.main.scene.find(&format!("{PANEL}/{path}")).with_context(|| format!("config {path}"))
    }

    /// `UISlider.value` with its thumb, as `ConfigCtrl.LoadConfigData` sets it.
    fn config_slider(&mut self, path: &str, v: f32) -> Result<()> {
        let node = self.config_node(path)?;
        let v = v.clamp(0., 1.);
        let fields = self.main.scene.script(node, "UISlider").cloned().unwrap_or_default();
        let bar = self.main.scene.script(node, "UISprite").or_else(|| self.main.scene.script(node, "UIWidget"));
        let (w, px) = bar.map(|f| (f.int("mWidth").unwrap_or(0) as f32, crate::ngui::pivot_x(f.int("mPivot").unwrap_or(4)))).unwrap_or((0., 0.5));
        if let Some(thumb) = fields.pptr("thumb").and_then(|t| self.main.scene.find_ref(node, t.path_id)) {
            let p = self.main.scene.nodes[thumb].position;
            self.main.states.entry(thumb).or_default().position = Some([(v - px) * w, p[1], p[2]]);
        }
        self.main.states.entry(node).or_default().value = Some(v);
        Ok(())
    }

    /// ConfigCtrl.SetButtonActive changes both buttons' default tint; hover is transient.
    fn shortcut_button_colors(&mut self) -> Result<()> {
        let on = self.config_node("System/SysButtonShowAlways/On")?;
        let off = self.config_node("System/SysButtonShowAlways/Off")?;
        let default = self.main.scene.script(on, "UISprite").and_then(|s| s.floats("mColor"))
            .and_then(|c| <[f32; 4]>::try_from(c).ok()).context("shortcut button color")?;
        for (node, selected) in [(on, self.cm_system.sys_button_show_always), (off, !self.cm_system.sys_button_show_always)] {
            let mut tint = default;
            if selected { tint[3] = 1.; }
            self.main.states.entry(node).or_default().color = Some(tint);
        }
        Ok(())
    }

    /// `ConfigMgr.OpenConfigPanel`: `LoadConfigData`, OK active, 0.5 s fade in.
    pub fn open_config(&mut self) -> Result<()> {
        if self.cm_system.open {
            return Ok(());
        }
        self.cm_system.open = true;
        for (path, handler) in SLIDERS.iter().chain([ENV_SLIDER].iter()).chain(BUTTONS.iter()) {
            if let Ok(n) = self.config_node(path) {
                self.main.states.entry(n).or_default().click = Some((*handler).to_owned());
            }
        }
        let ok = self.config_node("OK")?;
        self.main.states.entry(ok).or_default().click = Some("ConfigMgr.CloseConfigPanel".into());
        self.main.states.entry(ok).or_default().active = Some(true);
        // LoadConfigData sets each UIPopupList value, whose serialized
        // onChange calls UILabel.SetCurrentSelection on the child Label.
        self.shortcut_button_colors()?;
        let resolution = format!("{}x{}", crate::ngui::WIDTH, crate::ngui::HEIGHT);
        for (name, value) in [("Resolution", resolution.as_str()), ("AntiAlias", "X2"),
            ("ShadowQuality", "Medium"), ("TextureQuality", "High"), ("CaptureMagnification", "X1")] {
            let label = self.config_node(&format!("Screen/{name}/PopupList/Label"))?;
            self.main.states.entry(label).or_default().text = Some(value.to_owned());
        }
        let v = self.sound.volumes;
        let values = [
            self.man_alpha as f32,
            0.,
            self.msg_wnd_alpha,
            self.cm_system.text_speed as f32,
            self.cm_system.auto_speed as f32,
            self.sound.all_volume,
            v[Kind::System as usize],
            v[Kind::Bgm as usize],
            v[Kind::Voice as usize],
            v[Kind::Se as usize],
        ];
        for ((path, _), value) in SLIDERS.iter().zip(values) {
            self.config_slider(path, value / 100.)?;
        }
        self.config_slider(ENV_SLIDER.0, v[Kind::Env as usize] / 100.)?;
        let panel = self.main.scene.find(PANEL).context("ConfigPanel")?;
        self.main.states.entry(panel).or_default().active = Some(true);
        self.tweens.retain(|t| t.node != panel);
        self.tweens.push(crate::ops::Tween { node: panel, from: 0., to: 1., start: self.now, ms: 500, finished: None });
        Ok(())
    }

    /// A `ConfigMgr` handler of the panel's widgets.
    pub fn config_event(&mut self, node: usize, handler: &str) -> Result<()> {
        let value = self.main.states.get(&node).and_then(|s| s.value).unwrap_or(0.);
        // ConfigCtrl.ChangeSliderValue: Ceiling(value × 100).
        let num = (value * 100.).ceil();
        match handler {
            "ConfigMgr.CloseConfigPanel" => {
                // SetOKButtonActive(false), 0.5 s fade out, then inactive.
                if let Ok(ok) = self.config_node("OK") {
                    self.main.states.entry(ok).or_default().active = Some(false);
                }
                let panel = self.main.scene.find(PANEL).context("ConfigPanel")?;
                self.tweens.retain(|t| t.node != panel);
                self.tweens.push(crate::ops::Tween { node: panel, from: 1., to: 0., start: self.now, ms: 500, finished: Some("ConfigMgr.Closed".into()) });
            }
            "ConfigMgr.Closed" => {
                self.cm_system.open = false;
                let panel = self.main.scene.find(PANEL).context("ConfigPanel")?;
                self.main.states.entry(panel).or_default().active = Some(false);
            }
            "ConfigMgr.OnPlayerModelTransparencyChange" => self.man_alpha = num as i32,
            "ConfigMgr.OnMessageWindowTransparencyChange" => {
                self.msg_wnd_alpha = num;
                let bg = "__GameMain__/SystemUI Root/MessageWindowPanel/MessageViewer/MessageBox";
                if let Some(n) = self.main.scene.find(bg) {
                    let st = self.main.states.entry(n).or_default();
                    let mut c = st.color.unwrap_or([1.; 4]);
                    c[3] = (100. - num) / 100.;
                    st.color = Some(c);
                }
            }
            "ConfigMgr.OnTextSpeedChange" => self.cm_system.text_speed = num as i32,
            "ConfigMgr.OnAutoSpeedChange" => self.cm_system.auto_speed = num as i32,
            "ConfigMgr.OnMasterVolumeChange" => self.sound_ops.push(crate::sound::SoundOp::Volume { kind: None, value: num }),
            "ConfigMgr.OnSystemVolumeChange" => self.sound_ops.push(crate::sound::SoundOp::Volume { kind: Some(Kind::System), value: num }),
            "ConfigMgr.OnMusicVolumeChange" => self.sound_ops.push(crate::sound::SoundOp::Volume { kind: Some(Kind::Bgm), value: num }),
            "ConfigMgr.OnVoiceVolumeChange" => self.sound_ops.push(crate::sound::SoundOp::Volume { kind: Some(Kind::Voice), value: num }),
            "ConfigMgr.OnSoundEffectVolumeChange" => self.sound_ops.push(crate::sound::SoundOp::Volume { kind: Some(Kind::Se), value: num }),
            "ConfigMgr.OnSoundEnvironmentVolumeChange" => self.sound_ops.push(crate::sound::SoundOp::Volume { kind: Some(Kind::Env), value: num }),
            "ConfigMgr.OnSysButtonShowAlwaysEnabled" | "ConfigMgr.OnSysButtonShowAlwaysDisabled" => {
                self.cm_system.sys_button_show_always = handler == "ConfigMgr.OnSysButtonShowAlwaysEnabled";
                self.shortcut_button_colors()?;
            }
            other => self.log.push(format!("{other}: no host setting in this port (SATD-cm3d2-game-logic)")),
        }
        Ok(())
    }
}
